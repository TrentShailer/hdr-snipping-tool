use ash::vk::{
    Buffer, BufferCreateInfo, BufferUsageFlags, DeviceMemory, MemoryAllocateInfo,
    MemoryPropertyFlags, SharingMode,
};
use tracing::instrument;

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    /// Creates a basic unbound buffer with device memory backing it.
    #[instrument("VulkanInstance::create_unbound_buffer", skip_all, err)]
    pub fn create_unbound_buffer(
        &self,
        size: u64,
        usage: BufferUsageFlags,
        memory_flags: MemoryPropertyFlags,
    ) -> Result<(Buffer, DeviceMemory), VulkanError> {
        let (buffer, memory) = unsafe {
            let buffer_create_info = BufferCreateInfo::default()
                .size(size)
                .usage(usage)
                .sharing_mode(SharingMode::EXCLUSIVE);

            let buffer = self
                .device
                .create_buffer(&buffer_create_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating buffer"))?;

            let buffer_memory_requirements = self.device.get_buffer_memory_requirements(buffer);

            let buffer_memory_index = self
                .find_memorytype_index(&buffer_memory_requirements, memory_flags)
                .ok_or(VulkanError::NoSuitableMemoryType)?;

            let buffer_allocate_info = MemoryAllocateInfo::default()
                .allocation_size(buffer_memory_requirements.size)
                .memory_type_index(buffer_memory_index);

            let buffer_memory = self
                .device
                .allocate_memory(&buffer_allocate_info, None)
                .map_err(|e| VulkanError::VkResult(e, "allocating memory"))?;

            (buffer, buffer_memory)
        };

        Ok((buffer, memory))
    }

    /// Creates a basic bound buffer with device memory backing it.
    #[instrument("VulkanInstance::create_bound_buffer", skip_all, err)]
    pub fn create_bound_buffer(
        &self,
        size: u64,
        usage: BufferUsageFlags,
        memory_flags: MemoryPropertyFlags,
    ) -> Result<(Buffer, DeviceMemory), VulkanError> {
        let (buffer, memory) = self.create_unbound_buffer(size, usage, memory_flags)?;
        unsafe {
            self.device
                .bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| VulkanError::VkResult(e, "binding memory"))?
        }

        Ok((buffer, memory))
    }
}
