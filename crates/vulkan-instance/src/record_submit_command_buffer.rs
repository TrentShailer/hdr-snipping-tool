use std::time::{Duration, Instant};

use ash::{
    vk::{
        self, CommandBuffer, CommandBufferSubmitInfo, Fence, PipelineStageFlags2, Semaphore,
        SemaphoreSubmitInfo,
    },
    Device,
};
use smallvec::{smallvec, SmallVec};
use tracing::{info, instrument, Level};

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    #[instrument("VulkanInstance::record_submit_command_buffer", level = Level::DEBUG, skip_all, err)]
    pub fn record_submit_command_buffer<
        F: FnOnce(&Device, CommandBuffer) -> Result<(), VulkanError>,
    >(
        &self,
        command_buffer: (CommandBuffer, Fence),
        wait_semaphores: &[(Semaphore, PipelineStageFlags2)],
        signal_semaphores: &[(Semaphore, PipelineStageFlags2)],
        f: F,
    ) -> Result<(), VulkanError> {
        unsafe {
            let wait_start = Instant::now();
            self.device
                .wait_for_fences(&[command_buffer.1], true, u64::MAX)
                .map_err(|e| VulkanError::VkResult(e, "waiting for fences"))?;
            if wait_start.elapsed() > Duration::from_millis(1) {
                info!(
                    "waiting for fence: {:.2}ms",
                    wait_start.elapsed().as_secs_f64() * 1000.0
                );
            }

            self.device
                .reset_fences(&[command_buffer.1])
                .map_err(|e| VulkanError::VkResult(e, "resetting fences"))?;

            self.device
                .reset_command_buffer(
                    command_buffer.0,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .map_err(|e| VulkanError::VkResult(e, "resetting command buffer"))?;

            let command_buffer_begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            self.device
                .begin_command_buffer(command_buffer.0, &command_buffer_begin_info)
                .map_err(|e| VulkanError::VkResult(e, "beginning command buffer"))?;

            f(&self.device, command_buffer.0)?;

            self.device
                .end_command_buffer(command_buffer.0)
                .map_err(|e| VulkanError::VkResult(e, "ending command buffer"))?;

            // Submission:

            let command_buffer_submit_info =
                CommandBufferSubmitInfo::default().command_buffer(command_buffer.0);
            let command_buffer_submit_infos = &[command_buffer_submit_info];

            let mut wait_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 4]> = smallvec![];
            for (semaphore, stage) in wait_semaphores {
                let wait_semaphore_info = SemaphoreSubmitInfo::default()
                    .semaphore(*semaphore)
                    .stage_mask(*stage);
                wait_semaphore_infos.push(wait_semaphore_info);
            }

            let mut signal_semaphore_infos: SmallVec<[SemaphoreSubmitInfo; 3]> = smallvec![];
            for (semaphore, stage) in signal_semaphores {
                let signal_semaphore_info = SemaphoreSubmitInfo::default()
                    .semaphore(*semaphore)
                    .stage_mask(*stage);
                signal_semaphore_infos.push(signal_semaphore_info);
            }

            let submit_info = vk::SubmitInfo2::default()
                .wait_semaphore_infos(&wait_semaphore_infos)
                .signal_semaphore_infos(&signal_semaphore_infos)
                .command_buffer_infos(command_buffer_submit_infos);

            self.device
                .queue_submit2(self.queue, &[submit_info], command_buffer.1)
                .map_err(|e| VulkanError::VkResult(e, "submitting command buffer"))?;
        };

        Ok(())
    }
}
