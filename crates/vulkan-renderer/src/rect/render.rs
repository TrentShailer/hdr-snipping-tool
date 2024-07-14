use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    pipeline::Pipeline,
    ValidationError,
};

use crate::{
    pipelines::rect::vertex_shader::PushConstants,
    renderer::units::{VkPosition, VkSize},
};

use super::Rect;

impl Rect {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        position: VkPosition,
        size: VkSize,
    ) -> Result<(), Box<ValidationError>> {
        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants {
                    base_position: [0.0, 0.0],
                    base_size: [2.0, 2.0],
                    target_position: position.as_f32_array(),
                    target_size: size.as_f32_array(),
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        Ok(())
    }
}
