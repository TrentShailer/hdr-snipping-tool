use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyBufferInfo,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::graphics::vertex_input::Vertex,
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

/// Creates device local, not host accessable vertex and index buffers
/// from a set of verticies and indicies.
pub fn create_vertex_and_index_buffer<V: Vertex>(
    vk: &VulkanInstance,
    verticies: Vec<V>,
    indicies: Vec<u32>,
) -> Result<(Subbuffer<[V]>, Subbuffer<[u32]>), Error> {
    // verticies
    let vertex_buffer = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        verticies.len() as u64,
    )?;
    let vertex_staging = Buffer::from_iter(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        verticies,
    )?;

    // indicies
    let index_buffer = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::INDEX_BUFFER | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        indicies.len() as u64,
    )?;
    let index_staging = Buffer::from_iter(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        indicies,
    )?;

    // Copy from staging to buffer
    let mut builder = AutoCommandBufferBuilder::primary(
        &vk.allocators.command,
        vk.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .map_err(Error::CreateCommandBuffer)?;

    builder.copy_buffer(CopyBufferInfo::buffers(
        vertex_staging.clone(),
        vertex_buffer.clone(),
    ))?;
    builder.copy_buffer(CopyBufferInfo::buffers(
        index_staging.clone(),
        index_buffer.clone(),
    ))?;

    let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
    let future = sync::now(vk.device.clone())
        .then_execute(vk.queue.clone(), command_buffer)?
        .then_signal_fence_and_flush()
        .map_err(Error::SignalFenceAndFlush)?;
    future.wait(None).map_err(Error::AwaitFence)?;

    Ok((vertex_buffer, index_buffer))
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create buffers:\n{0:?}")]
    CreateBuffers(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0:?}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] Box<ValidationError>),

    #[error("Failed to signal fence and flush:\n{0:?}")]
    SignalFenceAndFlush(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),
}
