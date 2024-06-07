use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    pipeline::{Pipeline, PipelineBindPoint},
    swapchain::{acquire_next_image, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::{dpi::PhysicalPosition, window::Window};

use super::{fragment_shader::PushConstants, framebuffer, window_size_dependent_setup, Renderer};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to recreate swapchain:\n{0:?}")]
    RecreateSwapchain(#[source] Validated<VulkanError>),

    #[error("Failed to create framebuffer:\n{0}")]
    Framebuffer(#[from] framebuffer::Error),

    #[error("Failed to aquire image:\n{0:?}")]
    AquireImage(#[source] Validated<VulkanError>),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to write to command buffer:\n{0}")]
    UseCommandBuffer(#[from] Box<ValidationError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecuteCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to flush:\n{0:?}")]
    FailedToFlush(#[source] Validated<VulkanError>),
}

impl Renderer {
    pub fn render(
        &mut self,
        window: Arc<Window>,
        mouse_position: PhysicalPosition<i32>,
        selection: [u32; 4],
    ) -> Result<(), Error> {
        let image_extent: [u32; 2] = window.inner_size().into();
        // Don't try to render a surface that isn't visible
        if image_extent.contains(&0) {
            return Ok(());
        }

        // Don't try to render if we don't have a texture
        let texture = match self.texture.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Checks if the previous frame future has finished, if so, releases its resources
        // Non-blocking
        if let Some(prev_frame) = self.previous_frame_end.as_mut() {
            prev_frame.cleanup_finished();
        }

        // Handle recreatin the swapchain
        if self.recreate_swapchain {
            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent,
                    ..self.swapchain.create_info()
                })
                .map_err(Error::RecreateSwapchain)?;
            // TODO maybe handle ImageExtentNotSupported aparently
            // can happen while resizing

            self.swapchain = new_swapchain;

            // Because framebuffers contains a reference to the old swapchain, we need to
            // recreate framebuffers as well.
            self.framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_pass.clone(),
                &mut self.viewport,
            )?;

            self.recreate_swapchain = false;
        }

        // Returns a future that is cleared when the image is available
        let next_image_result = match acquire_next_image(self.swapchain.clone(), None) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                Validated::Error(_) => Err(Validated::unwrap(e)),
                Validated::ValidationError(_) => return Err(Error::AquireImage(e)),
            },
        };

        let (image_index, suboptimal, acquire_future) = match next_image_result {
            Ok(r) => r,
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                return Ok(());
            }
            Err(e) => return Err(Error::AquireImage(Validated::Error(e))),
        };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.cb_alloc,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            // Before we can draw, we have to *enter a render pass*.
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.02, 0.02, 0.02, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants {
                    mouse_position: [mouse_position.x, mouse_position.y].into(),
                    selection,
                },
            )?
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                texture.descriptor_set.clone(),
            )?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)?
            .end_render_pass(Default::default())?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;

        //
        let future = self
            .previous_frame_end
            .take()
            .unwrap_or_else(|| sync::now(self.device.clone()).boxed())
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)?
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        let future_result = match future {
            Ok(v) => Ok(v),
            Err(e) => match e {
                Validated::Error(_) => Err(Validated::unwrap(e)),
                Validated::ValidationError(_) => return Err(Error::FailedToFlush(e)),
            },
        };

        match future_result {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
                return Err(Error::FailedToFlush(Validated::Error(e)));
            }
        };

        Ok(())
    }
}
