use smallvec::{smallvec, SmallVec};
use thiserror::Error;
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{
        AutoCommandBufferBuilder, BufferCopy, CommandBufferExecError, CommandBufferUsage,
        CopyBufferInfo,
    },
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

use crate::VulkanInstance;

pub enum Region {
    SmallestBuffer,
    SingleRegion(BufferCopy),
    MultiRegion(SmallVec<[BufferCopy; 1]>),
}

/// Copies from a buffer to another and waits for the operation to complete
pub fn copy_buffer_and_wait(
    vk: &VulkanInstance,
    src: Subbuffer<impl ?Sized>,
    dst: Subbuffer<impl ?Sized>,
    region: Region,
) -> Result<(), Error> {
    let mut builder = AutoCommandBufferBuilder::primary(
        &vk.allocators.command,
        vk.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .map_err(Error::CreateCommandBuffer)?;

    let mut copy_info = CopyBufferInfo::buffers(src, dst);
    match region {
        Region::SmallestBuffer => {} // Default for constructor
        Region::SingleRegion(config) => copy_info.regions = smallvec![config],
        Region::MultiRegion(configs) => copy_info.regions = configs,
    };

    builder.copy_buffer(copy_info)?;

    let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
    let future = sync::now(vk.device.clone())
        .then_execute(vk.queue.clone(), command_buffer)?
        .then_signal_fence_and_flush()
        .map_err(Error::SignalFence)?;
    future.wait(None).map_err(Error::AwaitFence)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0:?}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] Box<ValidationError>),

    #[error("Failed to signal fence:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),
}
