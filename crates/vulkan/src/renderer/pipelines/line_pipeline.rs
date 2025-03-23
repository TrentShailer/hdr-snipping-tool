use core::slice;

use alloc::sync::Arc;
use ash::{ext, vk};
use ash_helper::{
    Context, LabelledVkResult, Swapchain, VkError, VulkanContext, link_shader_objects, try_name,
};
use bytemuck::bytes_of;

use crate::{
    RendererState, Vulkan,
    renderer::buffer::RenderBuffer,
    shaders::render_line::{self, Line, vertex_main::Vertex},
};

#[derive(Clone)]
pub struct LinePipeline {
    vulkan: Arc<Vulkan>,

    pub pipeline_layout: vk::PipelineLayout,
    pub shaders: Vec<vk::ShaderEXT>,
    pub stages: Vec<vk::ShaderStageFlags>,
}

impl LinePipeline {
    /// The verticies to build the selection shading, line list.
    pub const VERTICIES: [Vertex; 2] = [Vertex { index: 0 }, Vertex { index: 1 }];

    /// Create a new instance of the pipeline.
    pub unsafe fn new(vulkan: Arc<Vulkan>) -> LabelledVkResult<Self> {
        let pipeline_layout = {
            let push_range = Line::push_constant_range();

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .push_constant_ranges(slice::from_ref(&push_range));

            let layout = unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePiplineLayout"))?;

            unsafe { try_name(vulkan.as_ref(), layout, "Render Line Pipeline Layout") };

            layout
        };

        let (shaders, stages) = {
            let push_range = Line::push_constant_range();

            let vertex_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_line::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_line::vertex_main::STAGE)
                .name(render_line::vertex_main::ENTRY_POINT)
                .push_constant_ranges(slice::from_ref(&push_range));

            let fragment_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_line::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_line::fragment_main::STAGE)
                .name(render_line::fragment_main::ENTRY_POINT)
                .push_constant_ranges(slice::from_ref(&push_range));

            let mut create_infos = [vertex_create_info, fragment_create_info];

            let stages: Vec<_> = create_infos.iter().map(|info| info.stage).collect();
            let shaders = unsafe {
                link_shader_objects(vulkan.as_ref(), &mut create_infos, "RENDER LINE")
                    .map_err(|e| VkError::new(e, "vkCreateShadersEXT"))?
            };

            (shaders, stages)
        };

        Ok(Self {
            vulkan,
            pipeline_layout,
            shaders,
            stages,
        })
    }

    pub unsafe fn cmd_setup_draw(
        &self,
        command_buffer: vk::CommandBuffer,
        render_buffer: &RenderBuffer,
    ) {
        unsafe { self.cmd_set_state(command_buffer) };

        let shader_device: &ext::shader_object::Device = unsafe { self.vulkan.context() };

        // Bind shaders
        unsafe {
            shader_device.cmd_bind_shaders(command_buffer, &self.stages, &self.shaders);
        }

        // Bind buffers
        unsafe {
            self.vulkan.device().cmd_bind_vertex_buffers(
                command_buffer,
                0,
                slice::from_ref(&render_buffer.buffer),
                slice::from_ref(&render_buffer.line_offset),
            );
        }
    }

    pub unsafe fn cmd_draw_border(
        &self,
        command_buffer: vk::CommandBuffer,
        state: RendererState,
        swapchain: &Swapchain,
    ) {
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
        let top_line = Line {
            start: swapchain.screen_space([left_capped, top]),
            end: swapchain.screen_space([right_capped, top]),
            colour: border_colour,
        };
        let bottom_line = Line {
            start: swapchain.screen_space([left_capped, bottom]),
            end: swapchain.screen_space([right_capped, bottom]),
            colour: border_colour,
        };
        let left_line = Line {
            start: swapchain.screen_space([left, top_capped]),
            end: swapchain.screen_space([left, bottom_capped]),
            colour: border_colour,
        };
        let right_line = Line {
            start: swapchain.screen_space([right, top_capped]),
            end: swapchain.screen_space([right, bottom_capped]),
            colour: border_colour,
        };

        unsafe {
            self.cmd_draw(
                command_buffer,
                border_width,
                &[top_line, left_line, bottom_line, right_line],
            );
        }
    }

    pub unsafe fn cmd_draw_guides(
        &self,
        command_buffer: vk::CommandBuffer,
        state: RendererState,
        swapchain: &Swapchain,
    ) {
        let guide_colour = [0.5, 0.5, 0.5, 0.25];
        let mouse_position = swapchain.screen_space(state.mouse_position);

        let horizontal = Line {
            start: [mouse_position[0], -1.0],
            end: [mouse_position[0], 1.0],
            colour: guide_colour,
        };

        let vertical = Line {
            start: [-1.0, mouse_position[1]],
            end: [1.0, mouse_position[1]],
            colour: guide_colour,
        };

        unsafe { self.cmd_draw(command_buffer, 1.0, &[horizontal, vertical]) };
    }

    pub unsafe fn cmd_draw(
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
            // Push constants
            unsafe {
                self.vulkan.device().cmd_push_constants(
                    command_buffer,
                    self.pipeline_layout,
                    Line::STAGES,
                    0,
                    bytes_of(line),
                );
            }

            // Draw
            unsafe {
                self.vulkan.device().cmd_draw(
                    command_buffer,
                    Self::VERTICIES.len() as u32,
                    1,
                    0,
                    0,
                );
            }
        }
    }

    pub unsafe fn cmd_set_state(&self, command_buffer: vk::CommandBuffer) {
        let shader_device: &ext::shader_object::Device = unsafe { self.vulkan.context() };

        unsafe {
            shader_device.cmd_set_vertex_input(
                command_buffer,
                &render_line::vertex_main::vertex_binding_descriptions_2_ext(),
                &render_line::vertex_main::vertex_attribute_descriptions_2_ext(),
            );
        }

        unsafe {
            shader_device
                .cmd_set_primitive_topology(command_buffer, vk::PrimitiveTopology::LINE_LIST);
            shader_device.cmd_set_polygon_mode(command_buffer, vk::PolygonMode::LINE);
        }
    }
}

impl Drop for LinePipeline {
    fn drop(&mut self) {
        unsafe {
            let shader_device: &ext::shader_object::Device = self.vulkan.context();

            self.shaders
                .iter()
                .for_each(|shader| shader_device.destroy_shader(*shader, None));

            self.vulkan
                .device()
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
