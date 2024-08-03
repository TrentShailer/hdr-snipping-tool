use std::{sync::Arc, time::Duration};

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

use super::{window_size_dependent_setup, Renderer};

impl Renderer {
    pub fn render(
        &mut self,
        vk: &VulkanInstance,
        window: Arc<Window>,
        selection_top_left: [u32; 2],
        selection_size: [u32; 2],
        mouse_position: [u32; 2],
        should_wait: bool,
    ) -> Result<(), Error> {
        let window_size: [u32; 2] = window.inner_size().into();
        let window_scale = window.scale_factor();

        // Cleanup finished resources
        if let Some(v) = self.render_future.as_mut() {
            v.cleanup_finished();
        }

        // Don't try to render a surface that isn't visible
        if window_size.contains(&0) {
            return Ok(());
        }

        // Handle recreatin the swapchain
        if self.recreate_swapchain {
            self.recreate_swapchain(window_size)?;
        }

        let aquire_future = match self.aquire_future.take() {
            Some(v) => v,
            None => {
                // Get the next image index and future for when it is available
                let (_, suboptimal, acquire_future) =
                    match acquire_next_image(self.swapchain.clone(), None) {
                        Ok(v) => v,
                        Err(e) => {
                            if matches!(e, Validated::Error(VulkanError::OutOfDate)) {
                                self.recreate_swapchain = true;
                                return Ok(());
                            }

                            return Err(Error::AquireImage(e));
                        }
                    };

                if suboptimal {
                    self.recreate_swapchain = true;
                }

                acquire_future
            }
        };

        // if we shouldn't wait for the image and the image is not aquired, don't draw
        if !should_wait && aquire_future.wait(Some(Duration::from_secs(0))).is_err() {
            self.aquire_future = Some(aquire_future);
            return Ok(());
        }

        let image_index = aquire_future.image_index();

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
            self.capture
                .render(&mut builder)
                .map_err(|e| Error::Render(e, "capture"))?;

            self.mouse_guides
                .render(&mut builder, mouse_position, window_size, window_scale)
                .map_err(|e| Error::Render(e, "mouse_guides"))?;

            self.selection
                .render(
                    &mut builder,
                    selection_top_left,
                    selection_size,
                    window_size,
                    window_scale,
                )
                .map_err(|e| Error::Render(e, "selection"))?;
        }

        builder.end_rendering()?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;

        let render_future = self
            .render_future
            .take()
            .unwrap_or_else(|| sync::now(vk.device.clone()).boxed())
            .join(aquire_future)
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_swapchain_present(
                vk.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .boxed()
            .then_signal_fence_and_flush();

        match render_future {
            Ok(v) => {
                self.render_future = Some(v.boxed());
            }
            Err(e) => {
                if matches!(e, Validated::Error(VulkanError::OutOfDate)) {
                    self.recreate_swapchain = true;
                    self.render_future = Some(sync::now(vk.device.clone()).boxed());
                } else {
                    return Err(Error::FailedToFlush(e));
                }
            }
        };

        Ok(())
    }

    fn recreate_swapchain(&mut self, window_size: [u32; 2]) -> Result<(), Error> {
        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: window_size,
                ..self.swapchain.create_info()
            })
            .map_err(Error::RecreateSwapchain)?;
        self.swapchain = new_swapchain;

        let attachment_views = window_size_dependent_setup(&new_images, &mut self.viewport)?;
        self.attachment_views = attachment_views;

        self.aquire_future = None;

        self.recreate_swapchain = false;
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

    #[error("Failed to render {1}:\n{0}")]
    Render(#[source] Box<ValidationError>, &'static str),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecuteCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to flush:\n{0:?}")]
    FailedToFlush(#[source] Validated<VulkanError>),
}
