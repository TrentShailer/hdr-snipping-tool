use ash::Device;

use super::Capture;

impl Capture {
    pub fn drop(&self, device: &Device) {
        unsafe {
            device.destroy_buffer(self.vertex_buffer.0, None);
            device.free_memory(self.vertex_buffer.1, None);
            device.destroy_buffer(self.index_buffer.0, None);
            device.free_memory(self.index_buffer.1, None);

            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_sampler(self.sampler, None);
        }
    }
}
