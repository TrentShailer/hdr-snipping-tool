use core::slice;

use ash::vk;
use ash_helper::{
    cmd_transition_image, cmd_try_begin_label, cmd_try_end_label, FrameResources, LabelledVkResult,
    SurfaceContext, Swapchain, VkError, VulkanContext,
};

use crate::QueuePurpose;

use super::Renderer;

mod capture;
mod line;
mod selection;

impl Renderer {
    /// Render a frame.
    pub unsafe fn render(&mut self) -> LabelledVkResult<()> {
        // Recreate the swapchain if it needs recreating
        if self.swapchain.needs_to_rebuild {
            let swapchain = Swapchain::new(
                self.vulkan.as_ref(),
                &self.surface,
                self.vulkan.transient_pool(),
                self.vulkan.queue(QueuePurpose::Graphics),
                Some(self.swapchain.swapchain),
                &self.swapchain_preferences,
            )?;
            self.swapchain.destroy(self.vulkan.as_ref(), &self.surface);
            self.swapchain = swapchain;

            unsafe {
                self.line_pipeline
                    .recreate(&self.vulkan, self.swapchain.format)?;
                self.selection_pipeline
                    .recreate(&self.vulkan, self.swapchain.format)?;
                self.capture_pipeline
                    .recreate(&self.vulkan, self.swapchain.format)?;
            };
        }

        // Get frame resources
        let FrameResources {
            command_buffer,
            command_pool,
            in_flight_fence,
            image_available_semaphore,
            render_finished_semaphore,
        } = self.swapchain.current_resources(self.vulkan.as_ref())?;

        // Acquire next image
        let (image_index, image, image_view) = {
            let result = unsafe {
                self.surface.swapchain_device().acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    image_available_semaphore,
                    vk::Fence::null(),
                )
            };

            // If out of date, flag rebuild on next render
            let (image_index, suboptimal) = match result {
                Ok(v) => v,
                Err(e) => match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.swapchain.needs_to_rebuild = true;
                        return Ok(());
                    }

                    vk::Result::NOT_READY => return Ok(()),

                    e => return Err(VkError::new(e, "vkAcquireNextImageKHR")),
                },
            };

            if suboptimal {
                self.swapchain.needs_to_rebuild = true;
            }

            // Get the image & view
            let image_view = self.swapchain.views[image_index as usize];
            let image = self.swapchain.images[image_index as usize];

            (image_index, image, image_view)
        };

        // Reset in-flight fence
        unsafe {
            self.vulkan
                .device()
                .reset_fences(slice::from_ref(&in_flight_fence))
                .map_err(|e| VkError::new(e, "vkResetFences"))?;
        };

        // Commands
        {
            // Reset command pool
            unsafe {
                self.vulkan
                    .device()
                    .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                    .map_err(|e| VkError::new(e, "vkResetCommandPool"))?
            };

            // Start recording
            {
                let begin_info = vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

                unsafe {
                    self.vulkan
                        .device()
                        .begin_command_buffer(command_buffer, &begin_info)
                        .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;
                }

                cmd_try_begin_label(self.vulkan.as_ref(), command_buffer, "Render");
            }

            // Start rendering
            {
                // Transition swapchain image from present to colour attachment
                cmd_transition_image(
                    self.vulkan.as_ref(),
                    command_buffer,
                    image,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                )
                .unwrap();

                // Start rendering
                let colour_attachment = vk::RenderingAttachmentInfoKHR::default()
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.05, 0.05, 0.05, 1.0],
                        },
                    })
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .image_view(image_view)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE);

                let rendering_info = vk::RenderingInfoKHR::default()
                    .color_attachments(slice::from_ref(&colour_attachment))
                    .render_area(vk::Rect2D::default().extent(self.swapchain.extent))
                    .layer_count(1);

                unsafe {
                    self.surface
                        .rendering_device()
                        .cmd_begin_rendering(command_buffer, &rendering_info)
                };
            }

            // Set viewport & Scissor
            {
                let viewport = vk::Viewport::default()
                    .width(self.swapchain.extent.width as f32)
                    .height(self.swapchain.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0);
                self.vulkan.device().cmd_set_viewport(
                    command_buffer,
                    0,
                    slice::from_ref(&viewport),
                );

                let scissor = vk::Rect2D::default().extent(self.swapchain.extent);
                self.vulkan
                    .device()
                    .cmd_set_scissor(command_buffer, 0, slice::from_ref(&scissor));
            }

            // Draw
            {
                let state = *self.state.lock();
                self.cmd_draw_capture(command_buffer, state);
                self.cmd_draw_selection(command_buffer, state);
                self.cmd_draw_all_lines(command_buffer, state);
            }

            // End rendering
            {
                unsafe {
                    self.surface
                        .rendering_device()
                        .cmd_end_rendering(command_buffer)
                };

                // Transition swapchain image from present to colour attachment
                cmd_transition_image(
                    self.vulkan.as_ref(),
                    command_buffer,
                    image,
                    vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                )
                .unwrap();
            }

            // End recording
            unsafe {
                cmd_try_end_label(self.vulkan.as_ref(), command_buffer);

                self.vulkan
                    .device()
                    .end_command_buffer(command_buffer)
                    .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
            }
        }

        // Submit Render
        {
            let submit = vk::SubmitInfo::default()
                .command_buffers(slice::from_ref(&command_buffer))
                .wait_dst_stage_mask(slice::from_ref(
                    &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ))
                .wait_semaphores(slice::from_ref(&image_available_semaphore))
                .signal_semaphores(slice::from_ref(&render_finished_semaphore));

            let queue = self.vulkan.queue(QueuePurpose::Graphics).lock();
            self.vulkan
                .device()
                .queue_submit(*queue, slice::from_ref(&submit), in_flight_fence)
                .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
            drop(queue);
        }

        // Present frame
        {
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(slice::from_ref(&render_finished_semaphore))
                .swapchains(slice::from_ref(&self.swapchain.swapchain))
                .image_indices(slice::from_ref(&image_index));

            let queue = self.vulkan.queue(QueuePurpose::Graphics).lock();
            let result = self
                .surface
                .swapchain_device()
                .queue_present(*queue, &present_info);
            drop(queue);

            let suboptimal = match result {
                Ok(suboptimal) => suboptimal,
                Err(e) => match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => true,

                    e => return Err(VkError::new(e, "vkQueuePresentKHR")),
                },
            };

            if suboptimal {
                self.swapchain.needs_to_rebuild = true;
            }
        }

        self.swapchain.current_resources =
            (self.swapchain.current_resources + 1) % self.swapchain.max_frames_in_flight;

        Ok(())
    }
}
