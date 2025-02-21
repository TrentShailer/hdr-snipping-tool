use core::slice;

use ash::vk;
use ash_helper::VulkanContext;
use bytemuck::bytes_of;

use crate::{
    Renderer, RendererState,
    renderer::pipelines::{Line, LinePipeline},
};

impl Renderer {
    pub(super) unsafe fn cmd_draw_all_lines(
        &self,
        command_buffer: vk::CommandBuffer,
        state: RendererState,
    ) {
        unsafe {
            self.vulkan.device().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.line_pipeline.pipeline,
            );
        }

        unsafe {
            self.vulkan.device().cmd_bind_vertex_buffers(
                command_buffer,
                0,
                slice::from_ref(&self.render_buffer.buffer),
                slice::from_ref(&self.render_buffer.line_offset),
            );
        }

        unsafe { self.cmd_draw_border(command_buffer, state) };
        unsafe { self.cmd_draw_guides(command_buffer, state) };
    }

    unsafe fn cmd_draw_border(&self, command_buffer: vk::CommandBuffer, state: RendererState) {
        let border_width = 4.0;
        let border_colour = [1.0, 1.0, 1.0, 1.0];
        let selection = state.selection;

        let left = selection[0][0].min(selection[1][0]);
        let right = selection[0][0].max(selection[1][0]);
        let top = selection[0][1].min(selection[1][1]);
        let bottom = selection[0][1].max(selection[1][1]);

        let left_capped = left - border_width / 2.0;
        let right_capped = right + border_width / 2.0;
        let top_capped = top - border_width / 2.0;
        let bottom_capped = bottom + border_width / 2.0;

        // Cap the ends of the selection lines
        let top_line = Line::default()
            .start(self.swapchain.screen_space([left_capped, top]))
            .end(self.swapchain.screen_space([right_capped, top]))
            .colour(border_colour);
        let bottom_line = Line::default()
            .start(self.swapchain.screen_space([left_capped, bottom]))
            .end(self.swapchain.screen_space([right_capped, bottom]))
            .colour(border_colour);
        let left_line = Line::default()
            .start(self.swapchain.screen_space([left, top_capped]))
            .end(self.swapchain.screen_space([left, bottom_capped]))
            .colour(border_colour);
        let right_line = Line::default()
            .start(self.swapchain.screen_space([right, top_capped]))
            .end(self.swapchain.screen_space([right, bottom_capped]))
            .colour(border_colour);

        unsafe {
            self.cmd_draw_lines(
                command_buffer,
                border_width,
                &[top_line, left_line, bottom_line, right_line],
            );
        }
    }

    unsafe fn cmd_draw_guides(&self, command_buffer: vk::CommandBuffer, state: RendererState) {
        let guide_colour = [0.5, 0.5, 0.5, 0.25];
        let mouse = self.swapchain.screen_space(state.mouse_position);

        let horizontal = Line::default()
            .start([mouse[0], -1.0])
            .end([mouse[0], 1.0])
            .colour(guide_colour);

        let vertical = Line::default()
            .start([-1.0, mouse[1]])
            .end([1.0, mouse[1]])
            .colour(guide_colour);

        unsafe { self.cmd_draw_lines(command_buffer, 1.0, &[horizontal, vertical]) };
    }

    unsafe fn cmd_draw_lines(
        &self,
        command_buffer: vk::CommandBuffer,
        line_width: f32,
        lines: &[Line],
    ) {
        unsafe {
            self.vulkan
                .device()
                .cmd_set_line_width(command_buffer, line_width);
        }

        for line in lines {
            unsafe {
                self.vulkan.device().cmd_push_constants(
                    command_buffer,
                    self.line_pipeline.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytes_of(line),
                );
            }

            unsafe {
                self.vulkan.device().cmd_draw(
                    command_buffer,
                    LinePipeline::VERTICIES.len() as u32,
                    1,
                    0,
                    0,
                );
            }
        }
    }
}
