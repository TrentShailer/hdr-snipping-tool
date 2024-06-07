pub mod copy_to_cpu;

use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, layout::DescriptorSetLayout,
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        AllocateImageError, Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
    pipeline::Pipeline,
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;

use crate::VulkanInstance;

impl VulkanInstance {
    pub fn create_texture(&self, size: PhysicalSize<u32>) -> Result<Texture, Error> {
        Texture::new(
            self.device.clone(),
            self.queue.clone(),
            self.mem_alloc.clone(),
            self.cb_alloc.clone(),
            self.ds_alloc.clone(),
            self.renderer.pipeline.layout().set_layouts()[0].clone(),
            size,
        )
    }
}

pub struct Texture {
    pub image: Arc<Image>,
    pub image_view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
    pub size: PhysicalSize<u32>,
    pub descriptor_set: Arc<PersistentDescriptorSet>,
    mem_alloc: Arc<StandardMemoryAllocator>,
    cb_alloc: Arc<StandardCommandBufferAllocator>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl Texture {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        mem_alloc: Arc<StandardMemoryAllocator>,
        cb_alloc: Arc<StandardCommandBufferAllocator>,
        ds_alloc: Arc<StandardDescriptorSetAllocator>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        size: PhysicalSize<u32>,
    ) -> Result<Self, Error> {
        let extent = [size.width, size.height, 1];

        let image = Image::new(
            mem_alloc.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent,
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        let image_view = ImageView::new_default(image.clone()).map_err(Error::ImageView)?;

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .map_err(Error::Sampler)?;

        let descriptor_set = PersistentDescriptorSet::new(
            &ds_alloc,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, sampler.clone()),
                WriteDescriptorSet::image_view(1, image_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        Ok(Self {
            image,
            image_view,
            sampler,
            size,
            descriptor_set,
            cb_alloc,
            device,
            mem_alloc,
            queue,
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

    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
