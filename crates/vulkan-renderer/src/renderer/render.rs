use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    swapchain::{acquire_next_image, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::window::Window;

use crate::{
    glyph_cache::{self, GlyphCache},
    text,
};

use super::{window_size_dependent_setup, Renderer, BASE_FONT_SIZE};

impl Renderer {
    pub fn render(
        &mut self,
        vk: &VulkanInstance,
        window: Arc<Window>,
        selection_top_left: [u32; 2],
        selection_size: [u32; 2],
        mouse_position: [u32; 2],
    ) -> Result<(), Error> {
        let window_size: [u32; 2] = window.inner_size().into();
        let window_scale = window.scale_factor();

        if window_scale != self.window_scale {
            self.window_scale = window_scale;

            self.glyph_cache = GlyphCache::new(vk, BASE_FONT_SIZE * window_scale as f32)?;
            self.parameters
                .text
                .update_glyph_cache(vk, &mut self.glyph_cache)?;
        }

        // Checks if the previous frame future has finished, if so, releases its resources
        // Non-blocking
        if let Some(prev_frame) = self.previous_frame_end.as_mut() {
            prev_frame.cleanup_finished();
        }

        // Don't try to render a surface that isn't visible
        if window_size.contains(&0) {
            return Ok(());
        }

        // Handle recreatin the swapchain
        if self.recreate_swapchain {
            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: window_size,
                    ..self.swapchain.create_info()
                })
                .map_err(Error::RecreateSwapchain)?;

            self.swapchain = new_swapchain;

            // Because the attachment views are for the old swapchain, they must be recreated
            let attachment_views = window_size_dependent_setup(&new_images, &mut self.viewport)?;

            self.attachment_views = attachment_views;
            self.recreate_swapchain = false;
        }

        // Get the next image index and future for when it is available
        let (image_index, image_future) = {
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

            (image_index, acquire_future)
        };

        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: vulkano::render_pass::AttachmentLoadOp::Clear,
                    store_op: vulkano::render_pass::AttachmentStoreOp::Store,
                    clear_value: Some([0.05, 0.05, 0.05, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        self.attachment_views[image_index as usize].clone(),
                    )
                })],
                ..Default::default()
            })?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?;

        if self.capture.capture.is_some() {
            self.capture.render(&mut builder)?;

            self.mouse_guides
                .render(&mut builder, mouse_position, window_size, window_scale)?;

            self.selection.render(
                &mut builder,
                selection_top_left,
                selection_size,
                window_size,
                window_scale,
            )?;

            self.parameters.render(
                &mut builder,
                &self.glyph_cache,
                mouse_position,
                window_size,
                window_scale,
            )?;
        }

        builder.end_rendering()?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;

        let future = self
            .previous_frame_end
            .take()
            .unwrap_or_else(|| sync::now(vk.device.clone()).boxed())
            .join(image_future)
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_swapchain_present(
                vk.queue.clone(),
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
                self.previous_frame_end = Some(sync::now(vk.device.clone()).boxed());
            }
            Err(e) => {
                self.previous_frame_end = Some(sync::now(vk.device.clone()).boxed());
                return Err(Error::FailedToFlush(Validated::Error(e)));
            }
        };

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to recreate glyph cache:\n{0}")]
    GlyphCache(#[from] glyph_cache::Error),

    #[error("Failed to update glyph cache text:\n{0}")]
    UpdateGlyphCache(#[from] text::update_glyph_cache::Error),

    #[error("Failed to recreate swapchain:\n{0:?}")]
    RecreateSwapchain(#[source] Validated<VulkanError>),

    #[error("Failed to perform window size dependent setup:\n{0}")]
    WindowSizeDependentSetup(#[from] window_size_dependent_setup::Error),

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
