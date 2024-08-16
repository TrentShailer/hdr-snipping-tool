use std::sync::Arc;

use ash::{
    vk::{self, CommandBufferSubmitInfo, PipelineStageFlags2, SemaphoreSubmitInfo, SubmitFlags},
    Device,
};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use crate::{CommandBufferUsage, VulkanInstance};

impl VulkanInstance {
    pub fn record_submit_command_buffer<F: FnOnce(Arc<Device>, vk::CommandBuffer)>(
        &self,
        usage: CommandBufferUsage,
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        f: F,
    ) -> Result<(), Error> {
        unsafe {
            let fence = *self.fences.get(&usage).unwrap();
            let command_buffer = *self.command_buffers.get(&usage).unwrap();

            self.device
                .wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| Error::Vulkan(e, "waiting for fences"))?;

            self.device
                .reset_fences(&[fence])
                .map_err(|e| Error::Vulkan(e, "resetting fences"))?;

            self.device
                .reset_command_buffer(
                    command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .map_err(|e| Error::Vulkan(e, "resetting command buffer"))?;

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .map_err(|e| Error::Vulkan(e, "beginning command buffer"))?;

            f(self.device.clone(), command_buffer);

            self.device
                .end_command_buffer(command_buffer)
                .map_err(|e| Error::Vulkan(e, "ending command buffer"))?;

            // Submission:

            let command_buffer_submit_info = CommandBufferSubmitInfo {
                command_buffer,
                device_mask: 0,
                ..Default::default()
            };
            let command_buffers: SmallVec<[_; 1]> = smallvec![command_buffer_submit_info];

            let mut wait_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 4]> = smallvec![];
            for semaphore in wait_semaphores {
                let wait_semaphore_info = SemaphoreSubmitInfo {
                    semaphore: *semaphore,
                    stage_mask: PipelineStageFlags2::TOP_OF_PIPE,
                    device_index: 0,
                    ..Default::default()
                };
                wait_semaphore_infos.push(wait_semaphore_info);
            }

            let mut signal_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 3]> = smallvec![];
            for semaphore in signal_semaphores {
                let signal_semaphore_info = SemaphoreSubmitInfo {
                    semaphore: *semaphore,
                    stage_mask: PipelineStageFlags2::BOTTOM_OF_PIPE,
                    ..Default::default()
                };
                signal_semaphore_infos.push(signal_semaphore_info);
            }

            let submit_info = vk::SubmitInfo2 {
                flags: SubmitFlags::empty(),
                wait_semaphore_info_count: wait_semaphore_infos.len() as u32,
                p_wait_semaphore_infos: wait_semaphore_infos.as_ptr(),
                command_buffer_info_count: 1,
                p_command_buffer_infos: command_buffers.as_ptr(),
                signal_semaphore_info_count: signal_semaphore_infos.len() as u32,
                p_signal_semaphore_infos: signal_semaphore_infos.as_ptr(),
                ..Default::default()
            };

            self.device
                .queue_submit2(self.queue, &[submit_info], fence)
                .map_err(|e| Error::Vulkan(e, "submitting command buffer"))?;
        };

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),
}
