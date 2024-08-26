use tracing::{error, info_span};

use crate::VulkanInstance;

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        let _span = info_span!("VulkanInstance::Drop").entered();
        unsafe {
            if self.device.device_wait_idle().is_err() {
                error!("Failed to wait for device idle on drop");
                return;
            };

            self.device.destroy_fence(self.command_buffer.1, None);

            self.device
                .destroy_command_pool(self.command_buffer_pool, None);

            self.device.destroy_device(None);

            self.surface_loader.destroy_surface(self.surface, None);

            if let Some((debug_utils_loader, debug_messenger)) = &self.debug_utils {
                debug_utils_loader.destroy_debug_utils_messenger(*debug_messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
