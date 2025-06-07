use tracing::error;

use super::Vulkan;

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = self.device.device_wait_idle() {
                error!("Failed to wait for device idle: {e}");
            }

            let pool = self.transient_pool.lock();
            self.device.destroy_command_pool(*pool, None);
            drop(pool);

            self.device.destroy_device(None);

            if let Some(debug_utils) = self.debug_utils.as_ref() {
                debug_utils
                    .instance
                    .destroy_debug_utils_messenger(debug_utils.messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
