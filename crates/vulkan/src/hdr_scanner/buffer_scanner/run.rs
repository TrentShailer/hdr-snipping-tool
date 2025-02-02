use core::slice;

use ash::vk;
use ash_helper::{cmd_try_begin_label, cmd_try_end_label, VkError, VulkanContext};
use bytemuck::bytes_of;
use tracing::debug;

use crate::{
    hdr_scanner::{Error, ImageScanner, Resources, COMMAND_BUFFERS},
    vulkan::{QueuePurpose, Vulkan},
};

use super::{BufferScanner, PushConstants};

impl BufferScanner {
    pub unsafe fn run(
        &self,
        vulkan: &Vulkan,
        command_objects: &[(vk::CommandPool, vk::CommandBuffer)],
        semaphore: vk::Semaphore,
        semaphore_value: &mut u64,
        resources: &mut Resources,
    ) -> Result<(), Error> {
        let read_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(resources.buffer)
            .offset(0)
            .range(resources.read_size);

        let write_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(resources.buffer)
            .offset(resources.write_offset)
            .range(resources.write_size);

        let mut remaining_values =
            ImageScanner::output_count(resources.extent, resources.subgroup_size);

        let mut next_wait_value: u64 = *semaphore_value;
        let mut next_signal_value: u64 = *semaphore_value + 1;

        let mut data_in_read = true;
        let mut command_object_index = 1;

        let mut submission_count = 0;

        while remaining_values > 1 {
            // Wait for any work on the command buffer we want to use, to have completed
            'cb_guard: {
                // If no work has been done on this command buffer yet, no wait
                if COMMAND_BUFFERS as u64 >= next_signal_value {
                    break 'cb_guard;
                }

                let wait_value = next_signal_value - COMMAND_BUFFERS as u64;

                let wait_info = vk::SemaphoreWaitInfo::default()
                    .semaphores(slice::from_ref(&semaphore))
                    .values(slice::from_ref(&wait_value));

                unsafe { vulkan.device().wait_semaphores(&wait_info, u64::MAX) }
                    .map_err(|e| VkError::new(e, "vkWaitSemaphores"))?;
            }

            // Reset pool (buffer)
            let (command_pool, command_buffer) = command_objects[command_object_index];
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

                cmd_try_begin_label(
                    vulkan,
                    command_buffer,
                    &format!("HdrScanner Buffer Reduction {submission_count}"),
                );

                {
                    let push_constants = PushConstants {
                        input_length: remaining_values,
                    };
                    vulkan.device().cmd_push_constants(
                        command_buffer,
                        self.layout,
                        vk::ShaderStageFlags::COMPUTE,
                        0,
                        bytes_of(&push_constants),
                    );
                }

                {
                    let read_binding = if data_in_read { 0 } else { 1 };
                    let write_binding = if data_in_read { 1 } else { 0 };

                    let descriptor_writes = [
                        // Read buffer
                        vk::WriteDescriptorSet::default()
                            .dst_set(vk::DescriptorSet::null())
                            .dst_binding(read_binding)
                            .descriptor_count(1)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .buffer_info(slice::from_ref(&read_descriptor)),
                        // Write buffer
                        vk::WriteDescriptorSet::default()
                            .dst_set(vk::DescriptorSet::null())
                            .dst_binding(write_binding)
                            .descriptor_count(1)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .buffer_info(slice::from_ref(&write_descriptor)),
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

                vulkan.device().cmd_dispatch(
                    command_buffer,
                    Self::dispatch_count(remaining_values, resources.subgroup_size),
                    1,
                    1,
                );

                cmd_try_end_label(vulkan, command_buffer);

                vulkan
                    .device()
                    .end_command_buffer(command_buffer)
                    .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
            }

            // Submit work
            {
                let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                    .wait_semaphore_values(slice::from_ref(&next_wait_value))
                    .signal_semaphore_values(slice::from_ref(&next_signal_value));

                let submit_info = vk::SubmitInfo::default()
                    .wait_semaphores(slice::from_ref(&semaphore))
                    .signal_semaphores(slice::from_ref(&semaphore))
                    .command_buffers(slice::from_ref(&command_buffer))
                    .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COMPUTE_SHADER))
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

            // Update values
            remaining_values = Self::output_count(remaining_values, resources.subgroup_size);
            submission_count += 1;

            command_object_index = (command_object_index + 1) % COMMAND_BUFFERS;
            data_in_read = !data_in_read;

            next_wait_value = next_signal_value;
            next_signal_value += 1;
        }

        debug!("HdrScanner ran {} buffer reductions", submission_count);

        // Set result variables
        resources.result_in_read = data_in_read;
        *semaphore_value = next_wait_value;

        Ok(())
    }
}
