use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    ValidationError,
};

use crate::{
    glyph_cache::GlyphCache,
    renderer::units::{AddPhysical, FromPhysical, VkSize},
};

use super::Parameters;

impl Parameters {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        glyph_cache: &GlyphCache,
        mouse_position: [u32; 2],
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), Box<ValidationError>> {
        let text_size = VkSize::from_physical(self.text.extent, window_size);
        let text_position = self.get_text_position(mouse_position, window_size);

        let rect_size = text_size.add_physical([20, 10], window_size);

        self.rect.render(command_buffer, text_position, rect_size)?;

        self.border.render(
            command_buffer,
            text_position,
            rect_size,
            window_size,
            window_scale,
        )?;

        self.text
            .render(command_buffer, glyph_cache, text_position, window_size)?;

        Ok(())
    }
}
