use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    ValidationError,
};

use crate::{parameters_pipeline::vertex_shader::PushConstants, renderer::units::LogicalPosition};

use super::{text_renderer::FONT_SIZE, Parameters};

pub const TEXT_OFFSET: f32 = 64.0;

impl Parameters {
    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        position: LogicalPosition,
        window_size: [u32; 2],
    ) -> Result<(), Box<ValidationError>> {
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
                    font_size: FONT_SIZE,
                    window_size: [window_size[0] as f32, window_size[1] as f32],
                    text_position: position.into(),
                },
            )?
            .draw_indexed(self.index_buffer.len() as u32, self.instances, 0, 0, 0)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {}
