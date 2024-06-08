use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    swapchain::{acquire_next_image, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

use crate::VulkanInstance;

use super::{
    framebuffer, renderpass_border, renderpass_capture,
    renderpass_final::{self, RenderpassFinal},
    renderpass_mouse, renderpass_selection, window_size_dependent_setup, Renderer,
};

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

    #[error("Failed to render capture pass:\n{0}")]
    CapturePass(#[from] renderpass_capture::render::Error),

    #[error("Failed to render selection pass:\n{0}")]
    SelectionPass(#[from] renderpass_selection::render::Error),

    #[error("Failed to render border pass:\n{0}")]
    BorderPass(#[from] renderpass_border::render::Error),

    #[error("Failed to render mouse pass:\n{0}")]
    MousePass(#[from] renderpass_mouse::render::Error),

    #[error("Failed to render final pass:\n{0}")]
    FinalPass(#[from] renderpass_final::render::Error),

    #[error("Failed to create final pass attachmet set:\n{0}")]
    FinalAttachment(#[from] renderpass_final::attachment::Error),
}

impl Renderer {
    pub fn render(
        &mut self,
        vulkan: &VulkanInstance,
        window: Arc<Window>,
        mouse_position: PhysicalPosition<i32>,
        selection_ltrb: [u32; 4],
        capture_size: PhysicalSize<u32>,
    ) -> Result<(), Error> {
        let image_extent: [u32; 2] = window.inner_size().into();

        // Checks if the previous frame future has finished, if so, releases its resources
        // Non-blocking
        if let Some(prev_frame) = self.previous_frame_end.as_mut() {
            prev_frame.cleanup_finished();
        }

        // Don't try to render a surface that isn't visible
        if image_extent.contains(&0) {
            return Ok(());
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
            let (framebuffers, attachments) = window_size_dependent_setup(
                &vulkan,
                &new_images,
                self.render_pass.clone(),
                &mut self.viewport,
            )?;

            self.renderpass_final.attachment_set = RenderpassFinal::recreate_attachment_set(
                &vulkan,
                self.renderpass_final.pipeline.clone(),
                attachments.clone(),
            )?;

            self.framebuffers = framebuffers;
            self.attachments = attachments;

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
            &vulkan.allocators.command,
            vulkan.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            // Before we can draw, we have to *enter a render pass*.
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some([0.05, 0.05, 0.05, 1.0].into()),
                        None,
                        None,
                        None,
                        None,
                    ],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?;

        self.renderpass_capture.render(&mut builder)?;
        self.renderpass_selection
            .render(&mut builder, selection_ltrb, capture_size)?;
        self.renderpass_border
            .render(&mut builder, selection_ltrb, capture_size)?;
        self.renderpass_mouse
            .render(&mut builder, mouse_position, capture_size)?;
        self.renderpass_final.render(&mut builder)?;

        builder.end_render_pass(Default::default())?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;

        //
        let future = self
            .previous_frame_end
            .take()
            .unwrap_or_else(|| sync::now(vulkan.device.clone()).boxed())
            .join(acquire_future)
            .then_execute(vulkan.queue.clone(), command_buffer)?
            .then_swapchain_present(
                vulkan.queue.clone(),
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
                self.previous_frame_end = Some(sync::now(vulkan.device.clone()).boxed());
            }
            Err(e) => {
                self.previous_frame_end = Some(sync::now(vulkan.device.clone()).boxed());
                return Err(Error::FailedToFlush(Validated::Error(e)));
            }
        };

        Ok(())
    }
}
