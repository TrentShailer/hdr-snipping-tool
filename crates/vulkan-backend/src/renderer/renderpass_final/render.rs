use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    Validated, ValidationError, VulkanError,
};

use super::RenderpassFinal;

impl RenderpassFinal {
    pub fn render(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
    ) -> Result<(), Error> {
        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                self.attachment_set.clone(),
            )?
            .draw_indexed(6, 1, 0, 0, 0)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to write to command buffer:\n{0}")]
    UseCommandBuffer(#[from] Box<ValidationError>),

    #[error("Failed to bind descriptor set:\n{0:?}")]
    Descriptor(#[from] Validated<VulkanError>),
}
