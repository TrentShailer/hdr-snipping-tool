use ash::vk::{
    BufferUsageFlags, ImageView, MemoryPropertyFlags, PhysicalDeviceProperties2,
    PhysicalDeviceSubgroupProperties,
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
    let (read_buffer, read_buffer_memory) = vk
        .create_bound_buffer(
            buffer_length_bytes as u64,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .map_err(|e| Error::Vulkan(e, "creating read buffer"))?;

    // Setup "write" buffer
    let (write_buffer, write_buffer_memory) = vk
        .create_bound_buffer(
            buffer_length_bytes as u64,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .map_err(|e| Error::Vulkan(e, "creating write buffer"))?;

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
