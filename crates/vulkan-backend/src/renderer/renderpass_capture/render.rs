use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer, SubpassBeginInfo, SubpassContents, SubpassEndInfo,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    ValidationError,
};

use super::RenderpassCapture;

impl RenderpassCapture {
    pub fn render(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
    ) -> Result<(), Error> {
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
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?
            .next_subpass(
                SubpassEndInfo::default(),
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to write to command buffer:\n{0}")]
    UseCommandBuffer(#[from] Box<ValidationError>),
}
