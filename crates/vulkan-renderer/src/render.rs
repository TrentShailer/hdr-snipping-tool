use ash::vk::{
    AttachmentLoadOp, AttachmentStoreOp, ClearColorValue, ClearValue, Extent2D, ImageLayout,
    Offset2D, PipelineStageFlags2, PresentInfoKHR, Rect2D, RenderingAttachmentInfo, RenderingInfo,
    Semaphore,
};

use tracing::{instrument, Level};
use vulkan_instance::VulkanError;

use winit::window::Window;

use hdr_capture::Rect;

use super::Renderer;

impl Renderer {
    /// Try to queue a frame to be rendered and presented
    #[instrument("Renderer::render", level = Level::DEBUG, skip_all, err)]
    pub fn render(
        &mut self,
        window: &Window,
        mouse_position: [u32; 2],
        selection: Rect,
    ) -> Result<(), crate::Error> {
        let window_size: [u32; 2] = window.inner_size().into();
        let window_scale = window.scale_factor();

        // Don't try to render a surface that isn't visible
        if window_size.contains(&0) {
            return Ok(());
        }

        // Handle recreating the swapchain
        if self.recreate_swapchain {
            self.recreate_swapchain(window_size)?;
        }

        // try and find a free acquire fence
        let free_fence = self
            .acquire_fences
            .iter()
            .enumerate()
            .find_map(|(index, fence)| unsafe {
                match self.vk.device.get_fence_status(*fence) {
                    Ok(signalled) => {
                        if !signalled {
                            Some((index, *fence))
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            });

        let (sync_index, acquire_fence) = match free_fence {
            Some(values) => values,
            None => return Ok(()),
        };

        // 100μs
        let aquire_timeout = 100000;

        let image_index = unsafe {
            let aquire_result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                aquire_timeout,
                Semaphore::null(),
                acquire_fence,
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
                    ash::vk::Result::TIMEOUT => {
                        // TODO ensure fence is un-signalled for next run

                        return Ok(());
                    }
                    _ => {
                        return Err(crate::Error::Vulkan(VulkanError::VkResult(
                            e,
                            "aquiring image",
                        )))
                    }
                },
            }
        };

        // wait for and reset acquire fence
        unsafe {
            let fences = [acquire_fence];
            self.vk
                .device
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(|e| VulkanError::VkResult(e, "waiting for acquire fence"))?;
            self.vk
                .device
                .reset_fences(&fences)
                .map_err(|e| VulkanError::VkResult(e, "resetting acquire fence"))?;
        }

        let command_buffer = self.command_buffers[sync_index];
        let render_semaphore = self.render_semaphores[sync_index];

        self.vk.record_submit_command_buffer(
            command_buffer,
            &[],
            &[(render_semaphore, PipelineStageFlags2::BOTTOM_OF_PIPE)],
            |device, command_buffer| {
                unsafe {
                    let color_attachments = [RenderingAttachmentInfo::default()
                        .clear_value(ClearValue {
                            color: ClearColorValue {
                                float32: [0.05, 0.05, 0.05, 0.0],
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
                        self.capture
                            .render(
                                &self.capture_pipeline,
                                device,
                                command_buffer,
                                self.non_linear_swapchain,
                            )
                            .map_err(|e| VulkanError::VkResult(e, "rendering capture"))?;

                        self.mouse_guides
                            .render(
                                &self.mouse_guides_pipeline,
                                device,
                                command_buffer,
                                mouse_position,
                                window_size,
                                window_scale,
                            )
                            .map_err(|e| VulkanError::VkResult(e, "rendering mouse guides"))?;

                        self.selection
                            .render(
                                &self.border_pipeline,
                                &self.selection_shading_pipeline,
                                device,
                                command_buffer,
                                selection,
                                (window_size, window_scale),
                            )
                            .map_err(|e| VulkanError::VkResult(e, "rendering selection"))?;
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

        let suboptimal = unsafe {
            self.swapchain_loader
                .queue_present(self.vk.queue, &present_info)
        }
        .map_err(|e| VulkanError::VkResult(e, "queueing present"))?;

        if suboptimal {
            self.recreate_swapchain = true;
        }

        Ok(())
    }
}
