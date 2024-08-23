use ash::{
    util::Align,
    vk::{DeviceMemory, MemoryMapFlags},
};
use tracing::instrument;

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    /// Writes some data to some device memory. The device memory must be host visible and coherant.
    #[instrument("VulkanInstance::write_to_memory", skip_all, err)]
    pub unsafe fn write_to_memory<T: Copy>(
        &self,
        memory: DeviceMemory,
        data: &[T],
        offset: u64,
        size: u64,
    ) -> Result<(), VulkanError> {
        let memory_ptr = self
            .device
            .map_memory(memory, offset, size, MemoryMapFlags::empty())
            .map_err(|e| VulkanError::VkResult(e, "mapping memory"))?;

        let mut memory_slice = Align::new(memory_ptr, std::mem::align_of::<T>() as u64, size);
        memory_slice.copy_from_slice(data);

        self.device.unmap_memory(memory);

        Ok(())
    }

    /// Reads some data from device memory. The device memory must be host visible and coherant.
    #[instrument("VulkanInstance::read_from_memory", skip_all, err)]
    pub unsafe fn read_from_memory<T: Copy>(
        &self,
        memory: DeviceMemory,
        offset: u64,
        size: u64,
    ) -> Result<&[T], VulkanError> {
        let memory_ptr = self
            .device
            .map_memory(memory, offset, size, MemoryMapFlags::empty())
            .map_err(|e| VulkanError::VkResult(e, "mapping memory"))?;

        let data = std::slice::from_raw_parts(memory_ptr.cast(), size as usize);

        self.device.unmap_memory(memory);

        Ok(data)
    }
}
