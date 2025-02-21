use core::slice;

use ash::vk;
use ash_helper::VulkanContext;
use bytemuck::bytes_of;

use crate::{
    Renderer, RendererState,
    renderer::pipelines::{Selection, SelectionPipeline},
};

impl Renderer {
    pub(super) unsafe fn cmd_draw_selection(
        &self,
        command_buffer: vk::CommandBuffer,
        state: RendererState,
    ) {
        let physical_selection = state.selection;

        let start = self.swapchain.screen_space(physical_selection[0]);
        let end = self.swapchain.screen_space(physical_selection[1]);

        let selection = Selection { start, end };

        unsafe {
            self.vulkan.device().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.selection_pipeline.pipeline,
            );
        }

        unsafe {
            self.vulkan.device().cmd_bind_vertex_buffers(
                command_buffer,
                0,
                slice::from_ref(&self.render_buffer.buffer),
                slice::from_ref(&self.render_buffer.selection_offset),
            );
        }

        unsafe {
            self.vulkan.device().cmd_push_constants(
                command_buffer,
                self.selection_pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                bytes_of(&selection),
            );
        }
        unsafe {
            self.vulkan.device().cmd_draw(
                command_buffer,
                SelectionPipeline::VERTICIES.len() as u32,
                1,
                0,
                0,
            );
        }
    }
}
