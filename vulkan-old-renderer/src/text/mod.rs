mod gpu_data;

use std::{collections::HashMap, sync::Arc};

use fontdue::{Font, FontSettings, Metrics};
use gpu_data::InstanceData;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo},
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, GpuFuture},
};

use crate::vertex::Vertex;

const ATLAS_GLYPHS: &str = "0123456789:.AlphaGm "; // 20 chars

pub struct GlyphData {
    pub metrics: Metrics,
    pub atlas_offset: usize,
    pub atlas_length: usize,
}

pub struct TextRenderer {
    pub font: Font,
    pub atlas_image: Arc<Image>,
    pub atlas_sampler: Arc<Sampler>,
    pub atlas_view: Arc<ImageView>,
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub instance_buffer: Subbuffer<[InstanceData]>,
}

impl TextRenderer {
    pub fn new(vk: &VulkanInstance) -> Self {
        // TODO handle errors properly

        let font = include_bytes!("../fonts/FiraMono-Regular.ttf") as &[u8];
        let font = Font::from_bytes(font, FontSettings::default()).unwrap();

        // Build atlas
        let atlas_width = ATLAS_GLYPHS.len() * 16; // TODO maybe make atlas 2d?
        let atlas_height = 16;

        let mut atlas_data: Vec<u8> = vec![0; atlas_width * atlas_height];
        let mut glyph_map = HashMap::<char, GlyphData>::new();

        let mut altas_index = 0;
        for glyph in ATLAS_GLYPHS.chars() {
            if glyph_map.contains_key(&glyph) {
                continue;
            }

            let (metrics, bitmap) = font.rasterize(glyph, 16.0);
            let atlas_offset = altas_index;
            let atlas_length = bitmap.len();

            atlas_data[atlas_offset..(atlas_offset + atlas_length)].copy_from_slice(&bitmap);
            glyph_map.insert(
                glyph,
                GlyphData {
                    metrics,
                    atlas_offset,
                    atlas_length,
                },
            );
            altas_index += atlas_length;
        }

        // Create GPU atlas

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
        )
        .unwrap();

        staging_buffer.write().unwrap().copy_from_slice(&atlas_data);

        let image = Image::new(
            vk.allocators.memory.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8_UNORM,
                extent: [atlas_width as u32, atlas_height as u32, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        let image_view = ImageView::new_default(image.clone()).unwrap();

        let sampler = Sampler::new(
            vk.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .unwrap();

        // Copy to image
        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let copy_info = CopyBufferToImageInfo::buffer_image(staging_buffer.clone(), image.clone());

        builder.copy_buffer_to_image(copy_info).unwrap();

        let command_buffer = builder.build().unwrap();
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();
        future.wait(None).unwrap();

        Self {
            font,
            atlas_image: image,
            atlas_sampler: sampler,
            atlas_view: image_view,
        }
    }
}
