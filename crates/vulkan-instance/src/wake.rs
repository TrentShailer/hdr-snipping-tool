use tracing::instrument;

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    /// Tries to wake up the vulkan instance by submitting a small amount of work to a dedicated
    /// command buffer.
    #[instrument("VulkanInstance::wake", skip_all, err)]
    pub fn wake(&self) -> Result<(), VulkanError> {
        self.record_submit_command_buffer(
            self.wake_command_buffer,
            &[],
            &[],
            |_devive, _command_buffer| Ok(()),
        )?;

        Ok(())
    }
}
