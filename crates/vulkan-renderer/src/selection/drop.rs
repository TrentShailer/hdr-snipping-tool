use ash::Device;

use super::Selection;

impl Selection {
    pub fn drop(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.vertex_buffer.0, None);
            device.free_memory(self.vertex_buffer.1, None);
            device.destroy_buffer(self.index_buffer.0, None);
            device.free_memory(self.index_buffer.1, None);

            self.border.drop(device);
        }
    }
}
