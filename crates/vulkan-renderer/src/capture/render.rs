use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    ValidationError,
};

use crate::pipelines::capture::fragment_shader::PushConstants;

use super::Capture;

impl Capture {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
    ) -> Result<(), Box<ValidationError>> {
        let capture_ds = match self.capture_ds.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                capture_ds.clone(),
            )?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants {
                    whitepoint: self.whitepoint,
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        Ok(())
    }
}
