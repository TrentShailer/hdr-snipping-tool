use std::{collections::HashMap, sync::Arc};

use fontdue::{
    layout::{GlyphRasterConfig, Layout, TextStyle},
    Font, FontSettings, Metrics,
};
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyBufferToImageInfo,
    },
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        AllocateImageError, Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};
use winit::dpi::PhysicalPosition;

const ATLAS_GLYPHS: &str = "-0123456789:.AlphaGmMIxn ";
pub const FONT_SIZE: f32 = 32.0;

pub struct GlyphData {
    pub metrics: Metrics,
    pub index: PhysicalPosition<usize>,
}

pub struct TextRenderer {
    pub font: Arc<Font>,
    pub atlas: Arc<Image>,
    pub atlas_view: Arc<ImageView>,
    pub atlas_sampler: Arc<Sampler>,
    pub glyph_map: HashMap<GlyphRasterConfig, GlyphData>,
}

impl TextRenderer {
    pub fn new(vk: &VulkanInstance, layout: &mut Layout) -> Result<Self, Error> {
        let font = include_bytes!("../fonts/FiraMono-Regular.ttf") as &[u8];
        let font = Arc::from(
            Font::from_bytes(
                font,
                FontSettings {
                    scale: FONT_SIZE,
                    ..Default::default()
                },
            )
            .map_err(Error::Font)?,
        );

        // Build atlas
        let atlas_dim = (ATLAS_GLYPHS.chars().count() as f32).sqrt().ceil() as usize;

        let glyph_dim = FONT_SIZE.ceil() as usize;
        let atlas_width = atlas_dim * glyph_dim;
        let atlas_height = atlas_dim * glyph_dim;

        let mut atlas_data: Vec<u8> = vec![0; atlas_width * atlas_height];
        let mut glyph_map = HashMap::<GlyphRasterConfig, GlyphData>::new();
        let mut atlas_position: PhysicalPosition<usize> = PhysicalPosition::new(0, 0);

        layout.append(&[font.clone()], &TextStyle::new(ATLAS_GLYPHS, FONT_SIZE, 0));

        for glyph in layout.glyphs() {
            let (metrics, bitmap) = font.rasterize_config(glyph.key);

            // Because altas_data needs to be built row-wise, the data in the bitmap should be copied
            // into the atlas row by row with correct offsets.
            let glyph_data_start = (atlas_position.y * glyph_dim * glyph_dim * atlas_dim)
                + (atlas_position.x * glyph_dim);

            for row in 0..metrics.height {
                let glyph_row_offset = row * glyph_dim * atlas_dim;
                let data_start = glyph_data_start + glyph_row_offset;
                let row_start = row * metrics.width;

                atlas_data[data_start..(data_start + metrics.width)]
                    .copy_from_slice(&bitmap[row_start..(row_start + metrics.width)]);
            }

            glyph_map.insert(
                glyph.key,
                GlyphData {
                    metrics,
                    index: atlas_position,
                },
            );

            if atlas_position.x + 1 == atlas_dim {
                atlas_position.x = 0;
                atlas_position.y += 1;
            } else {
                atlas_position.x += 1;
            }
        }

        // Move Atlas to GPU
        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (atlas_width * atlas_height) as u64,
        )?;

        staging_buffer.write()?.copy_from_slice(&atlas_data);

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

        // Copy to image
        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        let copy_info = CopyBufferToImageInfo::buffer_image(staging_buffer.clone(), atlas.clone());

        builder.copy_buffer_to_image(copy_info)?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)
            .map_err(Error::ExecCommandBuffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        Ok(Self {
            atlas,
            atlas_sampler,
            atlas_view,
            font,
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
