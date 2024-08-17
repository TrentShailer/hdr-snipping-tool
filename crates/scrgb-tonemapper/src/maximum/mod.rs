use ash::vk::{
    BufferCreateInfo, BufferUsageFlags, ImageView, MemoryAllocateInfo, MemoryPropertyFlags,
    PhysicalDeviceProperties2, PhysicalDeviceSubgroupProperties, SharingMode,
};
use buffer_pass::buffer_reduction;
use half::f16;
use source_pass::source_reduction_pass;
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::VulkanInstance;

mod buffer_pass;
mod source_pass;

pub fn find_maximum(
    vk: &VulkanInstance,
    source: ImageView,
    source_size: [u32; 2],
) -> Result<f16, Error> {
    let _span = info_span!("find_maximum").entered();

    let subgroup_size = unsafe {
        let mut subgroup_properties = PhysicalDeviceSubgroupProperties::default();
        let mut physical_device_properties =
            PhysicalDeviceProperties2::default().push_next(&mut subgroup_properties);
        vk.instance
            .get_physical_device_properties2(vk.physical_device, &mut physical_device_properties);

        subgroup_properties.subgroup_size
    };

    // Buffer length = the number of dispatches * 2 bytes
    let dispatches_x = source_size[0].div_ceil(32);
    let dispatches_y = source_size[1].div_ceil(32).div_ceil(subgroup_size);
    let buffer_length_bytes = (dispatches_x * dispatches_y) * 2;

    // Setup "read" buffer
    let (read_buffer, read_buffer_memory) = unsafe {
        let buffer_create_info = BufferCreateInfo {
            size: buffer_length_bytes as u64,
            usage: BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = vk
            .device
            .create_buffer(&buffer_create_info, None)
            .map_err(|e| Error::Vulkan(e, "creating reading buffer"))?;

        let memory_requirements = vk.device.get_buffer_memory_requirements(buffer);

        let memory_index = vk
            .find_memorytype_index(&memory_requirements, MemoryPropertyFlags::DEVICE_LOCAL)
            .ok_or(Error::NoSuitableMemoryType)?;

        let allocate_info = MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: memory_index,
            ..Default::default()
        };

        let memory = vk
            .device
            .allocate_memory(&allocate_info, None)
            .map_err(|e| Error::Vulkan(e, "allocating reading buffer"))?;

        vk.device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|e| Error::Vulkan(e, "binding reading memory"))?;

        (buffer, memory)
    };

    // Setup "write" buffer
    let (write_buffer, write_buffer_memory) = unsafe {
        let buffer_create_info = BufferCreateInfo {
            size: buffer_length_bytes as u64,
            usage: BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = vk
            .device
            .create_buffer(&buffer_create_info, None)
            .map_err(|e| Error::Vulkan(e, "creating writing buffer"))?;

        let memory_requirements = vk.device.get_buffer_memory_requirements(buffer);

        let memory_index = vk
            .find_memorytype_index(&memory_requirements, MemoryPropertyFlags::DEVICE_LOCAL)
            .ok_or(Error::NoSuitableMemoryType)?;

        let allocate_info = MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index: memory_index,
            ..Default::default()
        };

        let memory = vk
            .device
            .allocate_memory(&allocate_info, None)
            .map_err(|e| Error::Vulkan(e, "allocating writing buffer"))?;

        vk.device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|e| Error::Vulkan(e, "binding writing memory"))?;

        (buffer, memory)
    };

    // Perform reduction on source writing results to read buffer
    source_reduction_pass(vk, source, source_size, read_buffer, subgroup_size)?;

    // finish reduction over read and write buffers until final result
    let maximum = buffer_reduction(
        vk,
        read_buffer,
        write_buffer,
        buffer_length_bytes,
        subgroup_size,
    )?;

    unsafe {
        vk.device.destroy_buffer(read_buffer, None);
        vk.device.free_memory(read_buffer_memory, None);

        vk.device.destroy_buffer(write_buffer, None);
        vk.device.free_memory(write_buffer_memory, None);
    }

    Ok(maximum)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to read shader:\n{0}")]
    ReadShader(#[source] std::io::Error),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] vulkan_instance::record_submit_command_buffer::Error),
}
