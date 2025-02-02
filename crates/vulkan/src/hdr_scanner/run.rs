use core::slice;
use std::time::Instant;

use ash::vk;
use ash_helper::{
    cmd_try_begin_label, cmd_try_end_label, queue_try_begin_label, queue_try_end_label, try_name,
    VkError, VulkanContext,
};
use half::f16;
use tracing::debug;

use crate::{HdrImage, QueuePurpose};

use super::{Error, HdrScanner};

impl HdrScanner {
    /// Find if an image contains HDR content based on the SDR white of the monitor the image is
    /// from.
    pub unsafe fn contains_hdr(
        &mut self,
        hdr_image: HdrImage,
        sdr_white: f32,
    ) -> Result<(bool, f32), Error> {
        let resources = self
            .resources
            .as_mut()
            .expect("HdrScanner::resources was None");

        // Record and submit image scan.
        self.image_scanner.run(
            &self.vulkan,
            &self.command_objects,
            self.semaphore,
            &mut self.semaphore_value,
            resources,
            hdr_image.view,
        )?;

        // Record and submit buffer scan.
        self.buffer_scanner.run(
            &self.vulkan,
            &self.command_objects,
            self.semaphore,
            &mut self.semaphore_value,
            resources,
        )?;

        // Get the result
        let maximum = {
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
                    "HdrScanner Result Command Buffer",
                );

                buffer
            };

            // find result offset
            let output_offset_bytes = if resources.result_in_read {
                0
            } else {
                resources.read_size
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
                    .size(size_of::<f16>() as u64)
                    .src_offset(output_offset_bytes)
                    .dst_offset(0);

                self.vulkan.device().cmd_copy_buffer(
                    command_buffer,
                    resources.buffer,
                    self.host_buffer,
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
                    queue_try_begin_label(self.vulkan.as_ref(), *queue, "HdrScanner Read Result");

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
                    "Waiting for HdrScanner took {}ms",
                    start.elapsed().as_millis()
                );
            }

            // Copy data to cpu
            let maximum = {
                let pointer = unsafe {
                    self.vulkan.device().map_memory(
                        self.host_memory,
                        0,
                        size_of::<f16>() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                }
                .map_err(|e| VkError::new(e, "vkMapMemory"))?;

                let raw_output: &[f16] = unsafe { slice::from_raw_parts(pointer.cast(), 1) };
                let maximum = raw_output[0];

                unsafe { self.vulkan.device().unmap_memory(self.host_memory) };

                maximum
            };

            // Free the buffer.
            unsafe {
                let pool = self.vulkan.transient_pool().lock();
                self.vulkan
                    .device()
                    .free_command_buffers(*pool, slice::from_ref(&command_buffer))
            };

            maximum
        };

        debug!("HDR Scanner found largest value: {:.3}", maximum);

        // The capture contains HDR if the largest colour component is > sdr_white.
        if maximum > f16::from_f32(sdr_white) {
            Ok((true, maximum.to_f32()))
        } else {
            Ok((false, maximum.to_f32()))
        }
    }
}
