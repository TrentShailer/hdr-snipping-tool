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
    pipelines::selection_shading::vertex_shader::PushConstants,
    renderer::units::{FromPhysical, VkPosition, VkSize},
};

use super::Selection;

impl Selection {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        selection_top_left: [u32; 2],
        selection_size: [u32; 2],
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), Box<ValidationError>> {
        let selection_top_left = VkPosition::from_physical(selection_top_left, window_size);
        let selection_size = VkSize::from_physical(selection_size, window_size);
        let selection_position = VkPosition::get_center(selection_top_left, selection_size);

        command_buffer
            .bind_pipeline_graphics(self.shading_pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .push_constants(
                self.shading_pipeline.layout().clone(),
                0,
                PushConstants {
                    base_position: [0.0, 0.0],
                    base_size: [1.0, 1.0],
                    target_position: selection_position.as_f32_array(),
                    target_size: selection_size.as_f32_array(),
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        self.border.render(
            command_buffer,
            selection_position,
            selection_size,
            window_size,
            window_scale,
        )?;

        Ok(())
    }
}
