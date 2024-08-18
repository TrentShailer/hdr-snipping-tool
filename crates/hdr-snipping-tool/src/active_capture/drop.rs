use windows::Win32::Foundation::CloseHandle;

use super::ActiveCapture;

impl Drop for ActiveCapture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.capture_view, None);
            self.device.destroy_image(self.capture_image, None);
            self.device.free_memory(self.capture_memory, None);
            CloseHandle(self.capture.handle).expect("Failed to close capture handle during drop");
        }
    }
}
