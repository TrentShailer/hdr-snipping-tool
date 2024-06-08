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
use winit::dpi::{PhysicalPosition, PhysicalSize};

use super::{fragment_shader, RenderpassMouse};

impl RenderpassMouse {
    pub fn render(
        &mut self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        mouse_position: PhysicalPosition<i32>,
        capture_size: PhysicalSize<u32>,
    ) -> Result<(), Error> {
        let x_clamped = mouse_position.x.clamp(1, capture_size.width as i32) as f32;
        let y_clamped = mouse_position.y.clamp(1, capture_size.height as i32) as f32;

        let x = x_clamped / capture_size.width as f32;
        let y = y_clamped / capture_size.height as f32;

        let x_limit = 1.0 / capture_size.width as f32;
        let y_limit = 1.0 / capture_size.height as f32;

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                fragment_shader::PushConstants {
                    mouse_position: [x, y],
                    limits: [x_limit, y_limit],
                },
            )?
            .draw_indexed(6, 1, 0, 0, 0)?
            .next_subpass(
                SubpassEndInfo::default(),
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?; // TODO get indicies from plane

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to write to command buffer:\n{0}")]
    UseCommandBuffer(#[from] Box<ValidationError>),
}
