use fontdue::layout::{GlyphRasterConfig, TextStyle};
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::HostAccessError,
    Validated,
};

use crate::{
    glyph_cache::{self, GlyphCache},
    pipelines::text::InstanceData,
};

use super::Text;

impl Text {
    pub fn set_text(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
        text: &str,
    ) -> Result<(), Error> {
        self.layout.clear();

        self.layout.append(
            &[glyph_cache.font.clone()],
            &TextStyle::new(text, glyph_cache.font_size, 0),
        );

        self.text = text.to_string();

        let mut text_right = 0.0;
        let mut instances: Vec<InstanceData> = vec![];

        let glyph_keys: Vec<GlyphRasterConfig> =
            self.layout.glyphs().iter().map(|g| g.key).collect();

        let glyph_data = glyph_cache.request_glyphs(vk, &glyph_keys)?;

        for (index, glyph) in self.layout.glyphs().iter().enumerate() {
            let glyph_data = glyph_data[index];

            let index = glyph_data.index;

            let uv_offset_x =
                (index.x * glyph_cache.glyph_dim) as f32 / glyph_cache.atlas.extent()[0] as f32;
            let uv_offset_y =
                (index.y * glyph_cache.glyph_dim) as f32 / glyph_cache.atlas.extent()[1] as f32;

            let instance = InstanceData {
                glyph_position: [glyph.x, glyph.y],
                glyph_size: [glyph.width as f32, glyph.height as f32],
                bitmap_size: [
                    glyph_data.metrics.width as f32,
                    glyph_data.metrics.height as f32,
                ],
                uv_offset: [uv_offset_x, uv_offset_y],
            };

            if glyph.x + glyph.width as f32 > text_right && glyph.char_data.rasterize() {
                text_right = glyph.x + glyph.width as f32;
            }

            instances.push(instance);
        }

        // resize buffer if instances is longer than buffer
        if instances.len() as u64 > self.instance_buffer.len() {
            self.instance_buffer = Buffer::new_slice(
                vk.allocators.memory.clone(),
                BufferCreateInfo {
                    usage: BufferUsage::VERTEX_BUFFER,
                    ..Default::default()
                },
                AllocationCreateInfo {
                    memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                        | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                    ..Default::default()
                },
                self.instance_buffer.len() * 2,
            )?;
        }

        self.instance_buffer.write()?[..instances.len()].copy_from_slice(&instances);
        self.instances = instances.len() as u32;
        self.extent = [text_right, self.layout.height()];

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to request glyphs from cache:\n{0}")]
    GlyphCache(#[from] glyph_cache::request_glyphs::Error),

    #[error("Failed to allocate new buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to write to buffer:\n{0}")]
    BufferWrite(#[from] HostAccessError),
}
