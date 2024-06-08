use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer, SubpassBeginInfo, SubpassContents, SubpassEndInfo,
    },
    pipeline::Pipeline,
    ValidationError,
};
use winit::dpi::PhysicalSize;

use super::{fragment_shader, RenderpassSelection};

impl RenderpassSelection {
    pub fn render(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        selection_ltrb: [u32; 4],
        capture_size: PhysicalSize<u32>,
    ) -> Result<(), Error> {
        let left = selection_ltrb[0] as f32 / capture_size.width as f32;
        let top = selection_ltrb[1] as f32 / capture_size.height as f32;
        let right = selection_ltrb[2] as f32 / capture_size.width as f32;
        let bottom = selection_ltrb[3] as f32 / capture_size.height as f32;

        let width_limit = 1.0 / capture_size.width as f32;
        let height_limit = 1.0 / capture_size.height as f32;

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                fragment_shader::PushConstants {
                    selection: [left, top, right, bottom],
                    limits: [width_limit, height_limit],
                },
            )?
            .draw_indexed(6, 1, 0, 0, 0)?
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
