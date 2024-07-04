pub mod request_glyphs;

use std::{collections::HashMap, sync::Arc};

use fontdue::{layout::GlyphRasterConfig, Font, FontSettings, Metrics};
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::AllocateBufferError,
    command_buffer::CommandBufferExecError,
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        AllocateImageError, Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::AllocationCreateInfo,
    sync::HostAccessError,
    Validated, ValidationError, VulkanError,
};
use winit::dpi::PhysicalPosition;

const ATLAS_DIM: usize = 32;

#[derive(Copy, Clone)]
pub struct GlyphData {
    pub metrics: Metrics,
    pub index: PhysicalPosition<usize>,
}

#[derive(Clone)]
pub struct GlyphCache {
    pub font: Arc<Font>,
    pub font_size: f32,
    pub glyph_dim: usize,
    pub atlas: Arc<Image>,
    pub atlas_view: Arc<ImageView>,
    pub atlas_sampler: Arc<Sampler>,
    pub glyph_map: HashMap<GlyphRasterConfig, GlyphData>,
    pub atlas_position: PhysicalPosition<usize>,
}

impl GlyphCache {
    pub fn new(vk: &VulkanInstance, font_size: f32) -> Result<Self, Error> {
        let font = include_bytes!("./fonts/FiraMono-Regular.ttf") as &[u8];
        let font = Arc::from(
            Font::from_bytes(
                font,
                FontSettings {
                    scale: 40.0,
                    ..Default::default()
                },
            )
            .map_err(Error::Font)?,
        );

        let glyph_map = HashMap::<GlyphRasterConfig, GlyphData>::new();

        let glyph_dim = font_size.ceil() as usize * 2; // TODO less naive solution
        let atlas_width = ATLAS_DIM * glyph_dim;
        let atlas_height = ATLAS_DIM * glyph_dim;

        let atlas = Image::new(
            vk.allocators.memory.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8_UNORM,
                extent: [atlas_width as u32, atlas_height as u32, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        let atlas_view = ImageView::new_default(atlas.clone()).map_err(Error::ImageView)?;

        let atlas_sampler = Sampler::new(
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
            font,
            font_size,
            glyph_dim,
            atlas,
            atlas_sampler,
            atlas_view,
            atlas_position: PhysicalPosition::new(0, 0),
            glyph_map,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load font:\n{0}")]
    Font(&'static str),

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffers(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create image:\n{0:?}")]
    CreateImage(#[from] Validated<AllocateImageError>),

    #[error("Failed to create image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),

    #[error("Failed to create sampler:\n{0:?}")]
    Sampler(#[source] Validated<VulkanError>),

    #[error("Failed to access buffer:\n{0}")]
    AccessBuffer(#[from] HostAccessError),

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
