use std::sync::Arc;

use ash::{
    vk::{self, CommandBufferSubmitInfo, PipelineStageFlags2, SemaphoreSubmitInfo},
    Device,
};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

use crate::{CommandBufferUsage, VulkanInstance};

impl VulkanInstance {
    pub fn record_submit_command_buffer<
        F: FnOnce(Arc<Device>, vk::CommandBuffer) -> Result<(), ash::vk::Result>,
    >(
        &self,
        usage: CommandBufferUsage,
        wait_semaphores: &[(vk::Semaphore, vk::PipelineStageFlags2)],
        signal_semaphores: &[(vk::Semaphore, vk::PipelineStageFlags2)],
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

            f(self.device.clone(), command_buffer)
                .map_err(|e| Error::Vulkan(e, "recording commands"))?;

            self.device
                .end_command_buffer(command_buffer)
                .map_err(|e| Error::Vulkan(e, "ending command buffer"))?;

            // Submission:

            let command_buffer_submit_info =
                CommandBufferSubmitInfo::default().command_buffer(command_buffer);
            let command_buffer_submit_infos = &[command_buffer_submit_info];

            let mut wait_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 4]> = smallvec![];
            for (semaphore, stage) in wait_semaphores {
                let wait_semaphore_info = SemaphoreSubmitInfo {
                    semaphore: *semaphore,
                    stage_mask: *stage,
                    device_index: 0,
                    ..Default::default()
                };
                wait_semaphore_infos.push(wait_semaphore_info);
            }

            let mut signal_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 3]> = smallvec![];
            for (semaphore, stage) in signal_semaphores {
                let signal_semaphore_info = SemaphoreSubmitInfo {
                    semaphore: *semaphore,
                    stage_mask: *stage,
                    ..Default::default()
                };
                signal_semaphore_infos.push(signal_semaphore_info);
            }

            let submit_info = vk::SubmitInfo2::default()
                .wait_semaphore_infos(&wait_semaphore_infos)
                .signal_semaphore_infos(&signal_semaphore_infos)
                .command_buffer_infos(command_buffer_submit_infos);

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
