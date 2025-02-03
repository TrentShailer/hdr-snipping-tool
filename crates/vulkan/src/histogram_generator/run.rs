use core::slice;
use std::time::Instant;

use ash::vk;
use ash_helper::{
    cmd_try_begin_label, cmd_try_end_label, queue_try_begin_label, queue_try_end_label, try_name,
    LabelledVkResult, VkError, VulkanContext,
};
use tracing::debug;

use crate::{HdrImage, QueuePurpose};

use super::{HistogramGenerator, BIN_COUNT};

impl HistogramGenerator {
    /// Generates a histogram for an HDR Image.
    pub unsafe fn generate(&mut self, image: HdrImage, maximum: f32) -> LabelledVkResult<Vec<u32>> {
        // Floating point precision issues hate this one simple trick
        let bin_width = maximum as f64 / BIN_COUNT as f64 + f64::EPSILON;

        let image_descriptor = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(image.view)
            .sampler(vk::Sampler::null());

        let buffer_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(self.buffer)
            .offset(0)
            .range(BIN_COUNT * 4);

        // Reset command pool
        unsafe {
            self.vulkan
                .device()
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| VkError::new(e, "vkResetCommandPool"))?;
        }

        // Write commands.
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.vulkan
                .device()
                .begin_command_buffer(self.command_buffer, &begin_info)
                .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

            cmd_try_begin_label(self.vulkan.as_ref(), self.command_buffer, "Histogram");

            // Zero initialize the buffer
            {
                self.vulkan.device().cmd_fill_buffer(
                    self.command_buffer,
                    self.buffer,
                    0,
                    BIN_COUNT * 4,
                    0,
                );
            }

            {
                let descriptor_writes = [
                    // Image
                    vk::WriteDescriptorSet::default()
                        .dst_set(vk::DescriptorSet::null())
                        .dst_binding(0)
                        .descriptor_count(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                        .image_info(slice::from_ref(&image_descriptor)),
                    // Buffer
                    vk::WriteDescriptorSet::default()
                        .dst_set(vk::DescriptorSet::null())
                        .dst_binding(1)
                        .descriptor_count(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(slice::from_ref(&buffer_descriptor)),
                ];

                self.vulkan
                    .push_descriptor_device()
                    .cmd_push_descriptor_set(
                        self.command_buffer,
                        vk::PipelineBindPoint::COMPUTE,
                        self.layout,
                        0,
                        &descriptor_writes,
                    );
            }

            self.vulkan.device().cmd_push_constants(
                self.command_buffer,
                self.layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                &bin_width.to_ne_bytes(),
            );

            self.vulkan.device().cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );

            // Dispatch
            {
                let dispatches_x = image.extent.width.div_ceil(16);
                let dispatches_y = image.extent.height.div_ceil(16);

                self.vulkan.device().cmd_dispatch(
                    self.command_buffer,
                    dispatches_x,
                    dispatches_y,
                    1,
                );
            }

            cmd_try_end_label(self.vulkan.as_ref(), self.command_buffer);

            self.vulkan
                .device()
                .end_command_buffer(self.command_buffer)
                .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
        }

        // Submit work
        {
            self.semaphore_value += 1;

            let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                .signal_semaphore_values(slice::from_ref(&self.semaphore_value));

            let submit_info = vk::SubmitInfo::default()
                .signal_semaphores(slice::from_ref(&self.semaphore))
                .command_buffers(slice::from_ref(&self.command_buffer))
                .push_next(&mut semaphore_submit_info);

            unsafe {
                let queue = self.vulkan.queue(QueuePurpose::Compute).lock();
                self.vulkan
                    .device()
                    .queue_submit(*queue, slice::from_ref(&submit_info), vk::Fence::null())
                    .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
                drop(queue);
            }
        }

        // Get the result
        let histogram = {
            let pool = self.vulkan.transient_pool().lock();

            // Allocate command buffer
            let command_buffer = {
                let allocate_info = vk::CommandBufferAllocateInfo::default()
                    .command_pool(*pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);

                let buffer = unsafe {
                    self.vulkan
                        .device()
                        .allocate_command_buffers(&allocate_info)
                }
                .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0];

                try_name(
                    self.vulkan.as_ref(),
                    buffer,
                    "Histogram Result Command Buffer",
                );

                buffer
            };

            // Record Copy
            unsafe {
                let begin_info = vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                self.vulkan
                    .device()
                    .begin_command_buffer(command_buffer, &begin_info)
                    .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

                cmd_try_begin_label(
                    self.vulkan.as_ref(),
                    command_buffer,
                    "HdrScanner Read Result",
                );

                let buffer_copy = vk::BufferCopy::default()
                    .size(BIN_COUNT * 4)
                    .src_offset(0)
                    .dst_offset(0);

                self.vulkan.device().cmd_copy_buffer(
                    command_buffer,
                    self.buffer,
                    self.staging_buffer,
                    slice::from_ref(&buffer_copy),
                );

                cmd_try_end_label(self.vulkan.as_ref(), command_buffer);

                self.vulkan
                    .device()
                    .end_command_buffer(command_buffer)
                    .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
            }

            // Submit Copy
            {
                let wait_value = self.semaphore_value;
                let signal_value = self.semaphore_value + 1;
                self.semaphore_value = signal_value;

                let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                    .wait_semaphore_values(slice::from_ref(&wait_value))
                    .signal_semaphore_values(slice::from_ref(&signal_value));

                let submit_info = vk::SubmitInfo::default()
                    .command_buffers(slice::from_ref(&command_buffer))
                    .wait_semaphores(slice::from_ref(&self.semaphore))
                    .signal_semaphores(slice::from_ref(&self.semaphore))
                    .push_next(&mut semaphore_submit_info)
                    .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::TRANSFER));

                unsafe {
                    let queue = self.vulkan.queue(QueuePurpose::Compute).lock();
                    queue_try_begin_label(self.vulkan.as_ref(), *queue, "Histogram Read Result");

                    self.vulkan
                        .device()
                        .queue_submit(*queue, slice::from_ref(&submit_info), vk::Fence::null())
                        .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;

                    queue_try_end_label(self.vulkan.as_ref(), *queue);
                    drop(queue);
                }
            }
            drop(pool);

            // Wait for submission to complete
            unsafe {
                let start = Instant::now();

                let wait_info = vk::SemaphoreWaitInfo::default()
                    .values(slice::from_ref(&self.semaphore_value))
                    .semaphores(slice::from_ref(&self.semaphore));

                self.vulkan
                    .device()
                    .wait_semaphores(&wait_info, u64::MAX)
                    .map_err(|e| VkError::new(e, "vkWaitSemaphores"))?;

                debug!(
                    "Waiting for Histogram took {}ms",
                    start.elapsed().as_millis()
                );
            }

            // Copy data to cpu
            let histogram = {
                let pointer = unsafe {
                    self.vulkan.device().map_memory(
                        self.staging_memory,
                        0,
                        BIN_COUNT * 4,
                        vk::MemoryMapFlags::empty(),
                    )
                }
                .map_err(|e| VkError::new(e, "vkMapMemory"))?;

                let histogram: Vec<u32> =
                    slice::from_raw_parts(pointer.cast(), BIN_COUNT as usize).to_vec();

                unsafe { self.vulkan.device().unmap_memory(self.staging_memory) };

                histogram
            };

            // Free the buffer.
            unsafe {
                let pool = self.vulkan.transient_pool().lock();
                self.vulkan
                    .device()
                    .free_command_buffers(*pool, slice::from_ref(&command_buffer))
            };

            histogram
        };

        Ok(histogram)
    }
}
