use std::sync::Arc;

use buffer_pass::buffer_reduction;
use half::f16;
use source_pass::source_reduction_pass;
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::{copy_buffer, VulkanInstance};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::CommandBufferExecError,
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::layout::IntoPipelineLayoutCreateInfoError,
    sync::HostAccessError,
    Validated, ValidationError, VulkanError,
};

mod buffer_pass;
mod source_pass;

pub fn find_maximum(
    vk: &VulkanInstance,
    source: Arc<ImageView>,
    source_size: [u32; 2],
) -> Result<f16, Error> {
    let _span = info_span!("find_maximum").entered();

    // Query the subgroup size from the GPU
    let subgroup_size = vk.physical_device.properties().subgroup_size.unwrap_or(1);
    let buffer_size = (source_size[0] * source_size[1])
        .div_ceil(32)
        .div_ceil(32)
        .div_ceil(subgroup_size)
        + 3;

    // Setup "read" buffer
    let read_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        buffer_size.into(),
    )?;

    // Setup "write" buffer
    let write_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        buffer_size.into(),
    )?;

    // Perform reduction on source writing results to read buffer
    source_reduction_pass(vk, source, source_size, read_buffer.clone())?;

    // finish reduction over read and write buffers until final result
    let maximum = buffer_reduction(vk, read_buffer, write_buffer, buffer_size * 2)?;

    Ok(maximum)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline layout info:\n{0:?}")]
    CreatePipelineLayoutInfo(#[from] IntoPipelineLayoutCreateInfoError),

    #[error("Failed to create pipeline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    CreateDescriptorSet(#[source] Validated<VulkanError>),

    #[error("Failed to access buffer:\n{0}")]
    BufferAccess(#[from] HostAccessError),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to write to command buffer:\n{0}")]
    WriteCommandBuffer(#[from] Box<ValidationError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to signal fence:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] copy_buffer::Error),
}
