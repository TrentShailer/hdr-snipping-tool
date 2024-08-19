use std::sync::Arc;

use ash::vk::{
    AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, Extent2D, Fence, ImageLayout,
    Offset2D, PipelineStageFlags2, PresentInfoKHR, Rect2D, RenderingAttachmentInfo, RenderingInfo,
    SemaphoreWaitInfo,
};
use thiserror::Error;
use tracing::info;
use vulkan_instance::{
    record_submit_command_buffer, CommandBufferUsage, SemaphoreUsage, VulkanInstance,
};

use winit::window::Window;

use super::{swapchain, Renderer};

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

        // Don't try to render a surface that isn't visible
        if window_size.contains(&0) {
            return Ok(());
        }

        // Handle recreatin the swapchain
        if self.recreate_swapchain {
            self.recreate_swapchain(vk, window_size)?;
        }

        let aquire_semaphore = *vk.semaphores.get(&SemaphoreUsage::Aquire).unwrap();
        let aquire_fence = *vk.fences.get(&CommandBufferUsage::AquireImage).unwrap();
        let render_semaphore = *vk.semaphores.get(&SemaphoreUsage::Render).unwrap();

        let aquire_timeout = if should_wait { u64::MAX } else { 0 };

        // try waiting for aquire fence
        // If the render call shouldn't wait and the fence is not signalled
        // then this render call is skipped
        // this ensures that when the render call can be passed through
        // it has the most up to date information
        unsafe {
            let fences = [aquire_fence];

            if let Err(e) = vk.device.wait_for_fences(&fences, true, aquire_timeout) {
                match e {
                    ash::vk::Result::TIMEOUT => return Ok(()),
                    _ => return Err(Error::Vulkan(e, "waiting for aquire fence")),
                }
            }

            vk.device
                .reset_fences(&fences)
                .map_err(|e| Error::Vulkan(e, "resetting aquire fence"))?;
        }

        let image_index = unsafe {
            let aquire_result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                aquire_timeout,
                aquire_semaphore,
                aquire_fence,
            );

            match aquire_result {
                Ok((image_index, suboptimal)) => {
                    if suboptimal {
                        self.recreate_swapchain = true;
                    }
                    image_index
                }
                Err(e) => match e {
                    ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.recreate_swapchain = true;
                        return Ok(());
                    }

                    ash::vk::Result::NOT_READY => return Ok(()),

                    // A wait operation has not completed in the specified time
                    ash::vk::Result::TIMEOUT => return Ok(()),
                    _ => return Err(Error::Vulkan(e, "aquiring image")),
                },
            }
        };

        vk.record_submit_command_buffer(
            CommandBufferUsage::Draw,
            &[(
                aquire_semaphore,
                PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            )],
            &[(render_semaphore, PipelineStageFlags2::BOTTOM_OF_PIPE)],
            |device, command_buffer| {
                unsafe {
                    // Wait for PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT?

                    let color_attachments = [RenderingAttachmentInfo::default()
                        .clear_value(ClearValue {
                            color: ClearColorValue {
                                float32: [0.05, 0.05, 0.05, 1.0],
                            },
                        })
                        .load_op(AttachmentLoadOp::CLEAR)
                        .store_op(AttachmentStoreOp::STORE)
                        .image_layout(ImageLayout::ATTACHMENT_OPTIMAL)
                        .image_view(self.attachment_views[image_index as usize])];
                    let render_area = Rect2D {
                        offset: Offset2D { x: 0, y: 0 },
                        extent: Extent2D {
                            width: window_size[0],
                            height: window_size[1],
                        },
                    };
                    let rendering_info = RenderingInfo::default()
                        .color_attachments(&color_attachments)
                        .layer_count(1)
                        .render_area(render_area);
                    device.cmd_begin_rendering(command_buffer, &rendering_info);

                    let viewports = [self.viewport];
                    device.cmd_set_viewport(command_buffer, 0, &viewports);
                    let scissors = [render_area];
                    device.cmd_set_scissor_with_count(command_buffer, &scissors);

                    if self.capture.loaded {
                        self.capture.render(&device, command_buffer)?;

                        self.mouse_guides.render(
                            &device,
                            command_buffer,
                            mouse_position,
                            window_size,
                            window_scale,
                        )?;

                        self.selection.render(
                            &device,
                            command_buffer,
                            selection_top_left,
                            selection_size,
                            window_size,
                            window_scale,
                        )?;
                    }

                    device.cmd_end_rendering(command_buffer);
                }

                Ok(())
            },
        )?;

        let wait_semaphores = [render_semaphore];
        let swapchains = [self.swapchain];
        let image_indicies = [image_index];
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indicies);

        let suboptimal = unsafe { self.swapchain_loader.queue_present(vk.queue, &present_info) }
            .map_err(|e| Error::Vulkan(e, "queueing present"))?;

        if suboptimal {
            self.recreate_swapchain = true;
        }

        // ------

        // Cleanup finished resources
        // if let Some(v) = self.render_future.as_mut() {
        //     v.cleanup_finished();
        // }

        // let render_future = self
        //     .render_future
        //     .take()
        //     .unwrap_or_else(|| sync::now(vk.device.clone()).boxed())
        //     .join(aquire_future)
        //     .then_execute(vk.queue.clone(), command_buffer)?
        //     .then_swapchain_present(
        //         vk.queue.clone(),
        //         SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
        //     )
        //     .boxed()
        //     .then_signal_fence_and_flush();

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to recreate swapchain:\n{0:?}")]
    RecreateSwapchain(#[from] swapchain::Error),

    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] record_submit_command_buffer::Error),
}
