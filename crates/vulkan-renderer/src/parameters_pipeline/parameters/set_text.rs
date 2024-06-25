use fontdue::layout::TextStyle;
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage},
    half::f16,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::HostAccessError,
    Validated,
};

use crate::parameters_pipeline::vertex::InstanceData;

use super::{text_renderer::FONT_SIZE, Parameters};

impl Parameters {
    pub fn update_parameters(
        &mut self,
        vk: &VulkanInstance,
        alpha: f16,
        gamma: f16,
        maximum: f16,
    ) -> Result<(), Error> {
        self.layout.reset(&Default::default());
        let text = format!(
            "Gamma: {:.2}\nAlpha: {:.2}\nInMax: {:.2}",
            gamma, alpha, maximum
        );

        self.layout.append(
            &[self.text_renderer.font.clone()],
            &TextStyle::new(&text, FONT_SIZE, 0),
        );

        let mut text_right = 0.0;
        let mut instances: Vec<InstanceData> = vec![];

        for glyph in self.layout.glyphs() {
            let glyph_data = match self.text_renderer.glyph_map.get(&glyph.key) {
                Some(v) => v,
                None => continue,
            };

            let index = glyph_data.index;

            let instance = InstanceData {
                position_offset: [glyph.x, glyph.y],
                size: [glyph.width as f32, glyph.height as f32],
                bitmap_size: [
                    glyph_data.metrics.width as f32,
                    glyph_data.metrics.height as f32,
                ],
                uv_offset: [
                    (index.x as f32 * FONT_SIZE.ceil())
                        / self.text_renderer.atlas.extent()[0] as f32,
                    (index.y as f32 * FONT_SIZE.ceil())
                        / self.text_renderer.atlas.extent()[1] as f32,
                ],
            };

            if glyph.x + glyph.width as f32 > text_right {
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
        self.text_right = text_right;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Input text is wrong length, {0} expected {1}")]
    InputLength(usize, usize),

    #[error("Input text contained char not in atlas '{0}'")]
    Char(char),

    #[error("Failed to allocate buffer:\n{0}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0}")]
    AccessBuffer(#[from] HostAccessError),
}
