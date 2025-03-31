use core::slice;

use ash::{ext, vk};
use ash_helper::{
    Context, FrameResources, LabelledVkResult, SurfaceContext, Swapchain, VkError, VulkanContext,
    cmd_transition_image, cmd_try_begin_label, cmd_try_end_label,
};
use tracing::debug;

use crate::QueuePurpose;

use super::Renderer;

impl Renderer {
    /// Render a frame.
    pub unsafe fn render(&mut self) -> LabelledVkResult<()> {
        // Recreate the swapchain if it needs recreating
        if self.swapchain.needs_to_rebuild {
            let swapchain = unsafe {
                Swapchain::new(
                    self.vulkan.as_ref(),
                    &self.surface,
                    self.vulkan.transient_pool(),
                    self.vulkan.queue(QueuePurpose::Graphics),
                    Some(self.swapchain.swapchain),
                    &self.swapchain_preferences,
                )?
            };

            unsafe { self.swapchain.destroy(self.vulkan.as_ref(), &self.surface) };

            debug!("Created {:?}", swapchain);

            self.swapchain = swapchain;
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
                    .map_err(|e| VkError::new(e, "vkResetCommandPool"))?;
            }

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

                unsafe { cmd_try_begin_label(self.vulkan.as_ref(), command_buffer, "Render") };
            }

            // Start rendering
            {
                // Transition swapchain image from present to colour attachment
                unsafe {
                    cmd_transition_image(
                        self.vulkan.as_ref(),
                        command_buffer,
                        image,
                        vk::ImageLayout::PRESENT_SRC_KHR,
                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    )
                    .unwrap();
                }

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
                        .cmd_begin_rendering(command_buffer, &rendering_info);
                }
            }

            // Set initial state
            unsafe {
                let shader_device: &ext::shader_object::Device = self.vulkan.context();

                let viewport = vk::Viewport::default()
                    .width(self.swapchain.extent.width as f32)
                    .height(self.swapchain.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0);
                shader_device
                    .cmd_set_viewport_with_count(command_buffer, slice::from_ref(&viewport));

                let scissor = vk::Rect2D::default().extent(self.swapchain.extent);
                shader_device.cmd_set_scissor_with_count(command_buffer, slice::from_ref(&scissor));

                shader_device.cmd_set_rasterizer_discard_enable(command_buffer, false);

                shader_device.cmd_set_primitive_restart_enable(command_buffer, false);

                shader_device
                    .cmd_set_rasterization_samples(command_buffer, vk::SampleCountFlags::TYPE_1);

                shader_device.cmd_set_sample_mask(
                    command_buffer,
                    vk::SampleCountFlags::TYPE_1,
                    &[u32::MAX],
                );

                shader_device.cmd_set_alpha_to_coverage_enable(command_buffer, false);

                shader_device.cmd_set_cull_mode(command_buffer, vk::CullModeFlags::NONE);
                shader_device.cmd_set_front_face(command_buffer, vk::FrontFace::COUNTER_CLOCKWISE);

                shader_device.cmd_set_depth_test_enable(command_buffer, false);
                shader_device.cmd_set_depth_bias_enable(command_buffer, false);
                shader_device.cmd_set_stencil_test_enable(command_buffer, false);

                shader_device.cmd_set_color_blend_enable(command_buffer, 0, &[vk::TRUE]);
                shader_device.cmd_set_color_write_mask(
                    command_buffer,
                    0,
                    &[vk::ColorComponentFlags::RGBA],
                );
                shader_device.cmd_set_color_blend_equation(
                    command_buffer,
                    0,
                    &[vk::ColorBlendEquationEXT::default()
                        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                        .color_blend_op(vk::BlendOp::ADD)
                        .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                        .alpha_blend_op(vk::BlendOp::ADD)],
                );

                shader_device.cmd_set_depth_write_enable(command_buffer, false);
            }

            // Draw
            {
                let state = *self.state.lock();
                unsafe {
                    self.capture_shader.cmd_draw(
                        command_buffer,
                        self.swapchain.format,
                        &self.render_buffer,
                        state,
                    )
                };
                unsafe {
                    self.selection_shader.cmd_draw(
                        command_buffer,
                        &self.swapchain,
                        &self.render_buffer,
                        state,
                    );
                };
                unsafe {
                    self.line_shader
                        .cmd_setup_draw(command_buffer, &self.render_buffer);
                    self.line_shader
                        .cmd_draw_border(command_buffer, state, &self.swapchain);
                    self.line_shader
                        .cmd_draw_guides(command_buffer, state, &self.swapchain);
                }
            }

            // End rendering
            {
                unsafe {
                    self.surface
                        .rendering_device()
                        .cmd_end_rendering(command_buffer);
                }

                // Transition swapchain image from present to colour attachment
                unsafe {
                    cmd_transition_image(
                        self.vulkan.as_ref(),
                        command_buffer,
                        image,
                        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        vk::ImageLayout::PRESENT_SRC_KHR,
                    )
                    .unwrap();
                }
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

            let queue = unsafe { self.vulkan.queue(QueuePurpose::Graphics).lock() };
            unsafe {
                self.vulkan
                    .device()
                    .queue_submit(*queue, slice::from_ref(&submit), in_flight_fence)
                    .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
            }
            drop(queue);
        }

        // Present frame
        {
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(slice::from_ref(&render_finished_semaphore))
                .swapchains(slice::from_ref(&self.swapchain.swapchain))
                .image_indices(slice::from_ref(&image_index));

            let queue = unsafe { self.vulkan.queue(QueuePurpose::Graphics).lock() };
            let result = unsafe {
                self.surface
                    .swapchain_device()
                    .queue_present(*queue, &present_info)
            };
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
