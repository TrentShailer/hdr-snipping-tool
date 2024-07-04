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
    pipelines::border::vertex_shader::PushConstants,
    renderer::units::{FromPhysical, VkPosition, VkSize},
};

use super::Border;

impl Border {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        position: VkPosition,
        size: VkSize,
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), Box<ValidationError>> {
        let line_size =
            VkSize::from_physical([self.line_size, self.line_size], window_size) * window_scale;

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
                    target_position: position.into(),
                    target_size: size.into(),
                    line_size: line_size.into(),
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        Ok(())
    }
}
