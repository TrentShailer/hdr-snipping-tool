use super::HdrCapture;

impl Drop for HdrCapture {
    fn drop(&mut self) {
        unsafe {
            self.vk.device.destroy_image_view(self.image_view, None);
            self.vk.device.destroy_image(self.image, None);
            self.vk.device.free_memory(self.memory, None);
        }
    }
}
