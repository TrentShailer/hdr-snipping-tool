use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    ValidationError,
};

use crate::{
    glyph_cache::GlyphCache,
    pipelines::text::vertex_shader::PushConstants,
    renderer::units::{FromPhysical, VkPosition, VkSize},
};

use super::Text;

impl Text {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        glyph_cache: &GlyphCache,
        position: VkPosition,
        window_size: [u32; 2],
    ) -> Result<(), Box<ValidationError>> {
        let size = VkSize::from_physical(self.extent, window_size);

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.atlas_ds.clone(),
            )?
            .bind_vertex_buffers(
                0,
                (self.vertex_buffer.clone(), self.instance_buffer.clone()),
            )?
            .bind_index_buffer(self.index_buffer.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants {
                    base_position: [0.0, 0.0],
                    base_size: [2.0, 2.0],

                    atlas_dim: glyph_cache.atlas.extent()[0] as f32,
                    window_size: [window_size[0] as f32, window_size[1] as f32],
                    text_position: position.as_f32_array(),
                    text_size: size.as_f32_array(),
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, self.instances, 0, 0, 0)?;

        Ok(())
    }
}
