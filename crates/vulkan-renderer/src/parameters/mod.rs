pub mod render;
pub mod set_parameters;

use std::sync::Arc;

use scrgb::ScRGB;
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::pipeline::GraphicsPipeline;

use crate::{
    border::Border,
    glyph_cache::GlyphCache,
    rect::Rect,
    renderer::units::{AddPhysical, FromPhysical, VkPosition, VkSize},
    text::{self, Text},
    vertex_index_buffer,
};

const TEXT_OFFSET: f32 = 64.0;

pub struct Parameters {
    pub text: Text,
    pub rect: Rect,
    pub border: Border,
    pub whitepoint: ScRGB,
}

impl Parameters {
    pub fn new(
        vk: &VulkanInstance,
        glyph_cache: &GlyphCache,
        text_pipeline: Arc<GraphicsPipeline>,
        rect_pipeline: Arc<GraphicsPipeline>,
        border_pipeline: Arc<GraphicsPipeline>,
    ) -> Result<Self, Error> {
        let text_color = [255, 255, 255, 235];
        let rect_color = [26, 32, 44, 255];
        let border_color = [23, 25, 35, 255];

        let text = Text::new(vk, text_pipeline, glyph_cache, 64, text_color)?;
        let rect =
            Rect::new(vk, rect_pipeline, rect_color).map_err(|e| Error::Object(e, "rect"))?;
        let border = Border::new(vk, border_pipeline, border_color, 1.0)
            .map_err(|e| Error::Object(e, "border"))?;

        Ok(Self {
            text,
            rect,
            border,
            whitepoint: ScRGB(0.0),
        })
    }

    pub fn get_text_position(&self, mouse_position: [u32; 2], window_size: [u32; 2]) -> VkPosition {
        let text_size = VkSize::from_physical(self.text.extent, window_size);
        let text_offset = VkSize::from_physical([TEXT_OFFSET, TEXT_OFFSET], window_size);
        let text_position = VkPosition::from([
            1.0 - text_size.x / 2.0 - text_offset.x,
            -1.0 + text_size.y / 2.0 + text_offset.y,
        ]);

        let mouse_position = VkPosition::from_physical(mouse_position, window_size);

        let text_bottom_left = VkPosition::from([
            text_position.x - text_size.x / 2.0,
            text_position.y + text_size.y / 2.0,
        ]);
        let obscured_bottom_left =
            text_bottom_left.add_physical([-TEXT_OFFSET * 4.0, TEXT_OFFSET * 4.0], window_size);

        let obscured =
            mouse_position.x > obscured_bottom_left.x && mouse_position.y < obscured_bottom_left.y;

        if obscured {
            VkPosition::from([
                -1.0 + text_size.x / 2.0 + text_offset.x,
                -1.0 + text_size.y / 2.0 + text_offset.y,
            ])
        } else {
            text_position
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create text:\n{0}")]
    Text(#[from] text::Error),

    #[error("Failed to create {1}:\n{0}")]
    Object(#[source] vertex_index_buffer::Error, &'static str),
}
