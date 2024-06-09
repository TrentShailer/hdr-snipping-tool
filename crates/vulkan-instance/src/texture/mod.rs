pub mod copy_to_vec;

use std::sync::Arc;

use thiserror::Error;
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
use winit::dpi::PhysicalSize;

use crate::VulkanInstance;

pub struct Texture {
    pub image: Arc<Image>,
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
    pub size: PhysicalSize<u32>,
}

impl Texture {
    pub fn new(vk: &VulkanInstance, size: PhysicalSize<u32>) -> Result<Self, Error> {
        let extent = [size.width, size.height, 1];

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
