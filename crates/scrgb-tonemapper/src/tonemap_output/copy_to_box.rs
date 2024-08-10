use thiserror::Error;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyImageToBufferInfo,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

use crate::VulkanInstance;

use super::TonemapOutput;

impl TonemapOutput {
    /// Copies the contents of the image to a box.
    pub fn copy_to_box(&self, vk: &VulkanInstance) -> Result<Box<[u8]>, Error> {
        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            self.size[0] as u64 * self.size[1] as u64 * 4,
        )?;

        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                self.image.clone(),
                staging_buffer.clone(),
            ))
            .map_err(Error::CopyImageToBuffer)?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::WaitFuture)?;

        let data_box = staging_buffer.read()?[..].to_owned().into_boxed_slice();

        Ok(data_box)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to copy image to staging buffer:\n{0}")]
    CopyImageToBuffer(#[source] Box<ValidationError>),

    #[error("Failed to signal fence and flush:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to wait for future:\n{0:?}")]
    WaitFuture(#[source] Validated<VulkanError>),

    #[error("Failed to read from staging buffer:\n{0}")]
    ReadStaging(#[from] HostAccessError),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecuteCommandBuffer(#[from] CommandBufferExecError),
}
