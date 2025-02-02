use core::slice;

use ash::vk;
use ash_helper::{cmd_try_begin_label, cmd_try_end_label, VkError, VulkanContext};

use crate::{
    hdr_scanner::{Error, Resources},
    vulkan::{QueuePurpose, Vulkan},
};

use super::ImageScanner;

impl ImageScanner {
    pub unsafe fn run(
        &self,
        vulkan: &Vulkan,
        command_objects: &[(vk::CommandPool, vk::CommandBuffer)],
        semaphore: vk::Semaphore,
        semaphore_value: &mut u64,
        resources: &mut Resources,
        image_view: vk::ImageView,
    ) -> Result<(), Error> {
        let image_descriptor = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(image_view)
            .sampler(vk::Sampler::null());

        let buffer_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(resources.buffer)
            .offset(0)
            .range(resources.read_size);

        // Reset pool (buffer)
        let (command_pool, command_buffer) = command_objects[0];
        unsafe {
            vulkan
                .device()
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .map_err(|e| VkError::new(e, "vkResetCommandPool"))?;
        }

        // Write commands.
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            vulkan
                .device()
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

            cmd_try_begin_label(vulkan, command_buffer, "ImageScanner Image Pass");

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

                vulkan.push_descriptor_device().cmd_push_descriptor_set(
                    command_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    self.layout,
                    0,
                    &descriptor_writes,
                );
            }

            vulkan.device().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline,
            );

            let dispatches = Self::dispatch_count(resources.extent);
            vulkan
                .device()
                .cmd_dispatch(command_buffer, dispatches[0], dispatches[1], 1);

            cmd_try_end_label(vulkan, command_buffer);

            vulkan
                .device()
                .end_command_buffer(command_buffer)
                .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
        }

        // Submit work
        {
            let signal_value = *semaphore_value + 1;
            *semaphore_value = signal_value;
            let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                .signal_semaphore_values(slice::from_ref(&signal_value));

            let submit_info = vk::SubmitInfo::default()
                .signal_semaphores(slice::from_ref(&semaphore))
                .command_buffers(slice::from_ref(&command_buffer))
                .push_next(&mut semaphore_submit_info);

            unsafe {
                let queue = vulkan.queue(QueuePurpose::Compute).lock();
                vulkan
                    .device()
                    .queue_submit(*queue, slice::from_ref(&submit_info), vk::Fence::null())
                    .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
                drop(queue);
            }
        }

        Ok(())
    }
}
