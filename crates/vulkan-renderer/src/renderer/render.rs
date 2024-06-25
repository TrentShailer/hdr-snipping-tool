use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    swapchain::{acquire_next_image, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::window::Window;

use super::{
    units::{LogicalPosition, LogicalScale},
    window_size_dependent_setup, Renderer,
};

impl Renderer {
    pub fn render(
        &mut self,
        vk: &VulkanInstance,
        window: Arc<Window>,
        selection: [u32; 4],
        mouse_position: [u32; 2],
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

        if self.capture.capture.is_none() {
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
            let framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_pass.clone(),
                &mut self.viewport,
            )?;

            self.framebuffers = framebuffers;

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
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder
            // Before we can draw, we have to *enter a render pass*.
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.05, 0.05, 0.05, 1.0].into())],
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

        let window_size: [u32; 2] = window.inner_size().into();

        self.capture.render(&mut builder)?;

        let selection_position =
            LogicalPosition::from_u32x2([selection[0], selection[1]], window_size);
        let selection_scale = LogicalScale::from_u32x2(
            [selection[2] - selection[0], selection[3] - selection[1]],
            window_size,
        );

        self.selection
            .render(&mut builder, selection_position, selection_scale)?;
        self.mouse.render(
            &mut builder,
            LogicalPosition::from_u32x2(mouse_position, window_size),
            window_size,
        )?;
        self.selection_border.render(
            &mut builder,
            selection_position,
            selection_scale,
            window_size,
        )?;

        // Text rendering
        let (text_position, text_size) = self
            .parameters
            .get_position_size(mouse_position, window_size);

        let rect_position = text_position.add_f32x2([-10.0, -5.0], window_size);
        let rect_scale = text_size.add_f32x2([20.0, 10.0], window_size);

        self.text_rect
            .render(&mut builder, rect_position, rect_scale)?;
        self.text_border
            .render(&mut builder, rect_position, rect_scale, window_size)?;
        self.parameters
            .render(&mut builder, text_position, window_size)?;

        builder.end_render_pass(Default::default())?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;

        //
        let future = self
            .previous_frame_end
            .take()
            .unwrap_or_else(|| sync::now(vk.device.clone()).boxed())
            .join(acquire_future)
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
