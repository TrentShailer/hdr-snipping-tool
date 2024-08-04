pub mod copy_to_box;

use std::sync::Arc;

use thiserror::Error;
use tracing::info_span;
use vulkano::{
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        AllocateImageError, Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::AllocationCreateInfo,
    Validated, VulkanError,
};

use crate::VulkanInstance;

/// Output from the tonemapper.\
/// Vulkan image and associated values.
pub struct TonemapOutput {
    pub image: Arc<Image>,
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
    pub size: [u32; 2],
}

impl TonemapOutput {
    /// Create an empty tonemap output.
    pub fn new(vk: &VulkanInstance, size: [u32; 2]) -> Result<Self, Error> {
        let _span = info_span!("TonemapOutput::new").entered();

        let extent = [size[0], size[1], 1];

        let image = Image::new(
            vk.allocators.memory.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent,
                usage: ImageUsage::TRANSFER_SRC
                    | ImageUsage::TRANSFER_DST
                    | ImageUsage::SAMPLED
                    | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        let image_view = ImageView::new_default(image.clone()).map_err(Error::ImageView)?;

        let sampler = Sampler::new(
            vk.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .map_err(Error::Sampler)?;

        Ok(Self {
            image,
            image_view,
            sampler,
            size,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create image:\n{0:?}")]
    NewImage(#[from] Validated<AllocateImageError>),

    #[error("Failed to create image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),

    #[error("Failed to create image sampler:\n{0:?}")]
    Sampler(#[source] Validated<VulkanError>),
}
