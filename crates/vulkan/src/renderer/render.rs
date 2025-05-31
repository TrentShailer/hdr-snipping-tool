use core::slice;

use ash::{ext, vk};
use ash_helper::{
    Context, Frame, FrameResources, LabelledVkResult, Swapchain, VkError, VulkanContext,
    cmd_transition_image, cmd_try_begin_label, cmd_try_end_label, onetime_command,
};
use tracing::debug;
use utilities::DebugTime;

use crate::QueuePurpose;

use super::Renderer;

impl Renderer {
    /// Render a frame.
    pub unsafe fn render(&mut self) -> LabelledVkResult<()> {
        self.swapchain_retirement
            .process_retirement(self.vulkan.as_ref(), &self.surface)?;

        // Recreate the swapchain if it needs recreating
        if self.swapchain.needs_to_rebuild {
            let _timing = DebugTime::start("Recreate Swapchain");

            let new_swapchain = {
                let create_info = self
                    .swapchain_preferences
                    .get_swapchain_create_info(self.vulkan.as_ref(), &self.surface)?;

                unsafe {
                    Swapchain::new(
                        self.vulkan.as_ref(),
                        &self.surface,
                        Some(&mut self.swapchain),
                        create_info,
                    )?
                }
            };

            debug!("Created {new_swapchain:?}");

            let old_swapchain = core::mem::replace(&mut self.swapchain, new_swapchain);

            self.swapchain_retirement.house_swapchain(old_swapchain);
        }

        // Acquire next image
        let Frame {
            image_index,
            image,
            view,
            resources,
            previously_acquired: _,
        } = {
            let acquire_fence = self.swapchain_retirement.get_fence(self.vulkan.as_ref())?;

            let result = self.swapchain.acquire_next_image(
                self.vulkan.as_ref(),
                &self.surface,
                acquire_fence,
            )?;

            let frame = match result {
                Some(frame) => frame,
                None => return Ok(()),
            };

            self.swapchain_retirement.track_acquisition(
                self.swapchain.swapchain,
                acquire_fence,
                frame.image_index,
            );

            if !frame.previously_acquired {
                unsafe {
                    onetime_command(
                        self.vulkan.as_ref(),
                        frame.resources.command_pool,
                        self.vulkan.queue(QueuePurpose::Graphics),
                        |vulkan, command_buffer| {
                            cmd_transition_image(
                                vulkan,
                                command_buffer,
                                frame.image,
                                vk::ImageLayout::UNDEFINED,
                                vk::ImageLayout::PRESENT_SRC_KHR,
                            )
                            .unwrap();
                        },
                        "Transition swapchain image",
                    )?;
                }
            }

            frame
        };

        let FrameResources {
            acquire_semaphore,
            render_semaphore,
            render_fence,
            command_pool,
            command_buffer,
        } = resources;

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
                    .image_view(view)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE);

                let rendering_info = vk::RenderingInfoKHR::default()
                    .color_attachments(slice::from_ref(&colour_attachment))
                    .render_area(vk::Rect2D::default().extent(self.swapchain.info.extent))
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
                    .width(self.swapchain.info.extent.width as f32)
                    .height(self.swapchain.info.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0);
                shader_device
                    .cmd_set_viewport_with_count(command_buffer, slice::from_ref(&viewport));

                let scissor = vk::Rect2D::default().extent(self.swapchain.info.extent);
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
                        self.swapchain.info.format,
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
                .wait_semaphores(slice::from_ref(&acquire_semaphore))
                .signal_semaphores(slice::from_ref(&render_semaphore));

            let queue = unsafe { self.vulkan.queue(QueuePurpose::Graphics).lock() };
            unsafe {
                self.vulkan
                    .device()
                    .queue_submit(*queue, slice::from_ref(&submit), render_fence)
                    .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
            }
            drop(queue);
        }

        // Present frame
        {
            let queue = unsafe { self.vulkan.queue(QueuePurpose::Graphics).lock() };

            self.swapchain
                .queue_present(&self.surface, image_index, render_semaphore, *queue)?;

            drop(queue);
        }

        Ok(())
    }
}
