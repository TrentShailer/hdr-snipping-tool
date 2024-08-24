use tracing::info_span;

use super::HdrCapture;

impl Drop for HdrCapture {
    fn drop(&mut self) {
        let _span = info_span!("HdrCapture::Drop").entered();
        unsafe {
            self.vk.device.device_wait_idle().unwrap();
            self.vk.device.destroy_image_view(self.image_view, None);
            self.vk.device.destroy_image(self.image, None);
            self.vk.device.free_memory(self.memory, None);
        }
    }
}
