use std::sync::Arc;

use thiserror::Error;
use tracing::info_span;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyBufferToImageInfo,
    },
    format::Format,
    image::{view::ImageView, AllocateImageError, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};
use windows_capture_provider::Capture;

use super::ActiveCapture;

impl ActiveCapture {
    pub fn image_from_capture(
        vk: &VulkanInstance,
        capture: &Capture,
    ) -> Result<Arc<ImageView>, Error> {
        let _span = info_span!("image_from_capture").entered();

        let extent = [capture.display.size[0], capture.display.size[1], 1];

        let image = Image::new(
            vk.allocators.memory.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R16G16B16A16_SFLOAT,
                extent,
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        // Transfer capture to staging buffer
        let staging_span = info_span!("write_staging").entered();
        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            capture.data.len() as u64,
        )?;
        staging_buffer.write()?.copy_from_slice(&capture.data);
        staging_span.exit();

        // Copy from buffer to image
        let gpu_span = info_span!("write_gpu").entered();
        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging_buffer.clone(),
                image.clone(),
            ))
            .map_err(Error::CopyBufferToImage)?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFenceAndFlush)?;
        future.wait(None).map_err(Error::AwaitFence)?;
        gpu_span.exit();

        let image_view = ImageView::new_default(image).map_err(Error::ImageView)?;
        Ok(image_view)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create image:\n{0:?}")]
    CreateImage(#[from] Validated<AllocateImageError>),

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0}")]
    AccessBuffer(#[from] HostAccessError),

    #[error("Failed to create to command buffer:\n{0}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to signal fence and flush:\n{0:?}")]
    SignalFenceAndFlush(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),

    #[error("Failed to copy buffer to image:\n{0}")]
    CopyBufferToImage(#[source] Box<ValidationError>),

    #[error("Failed to create image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),
}
