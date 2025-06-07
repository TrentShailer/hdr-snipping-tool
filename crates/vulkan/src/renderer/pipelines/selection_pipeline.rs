use alloc::sync::Arc;
use core::slice;

use ash::{ext, vk};
use ash_helper::{
    Context, LabelledVkResult, Swapchain, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext,
    link_shader_objects, try_name,
};
use bytemuck::bytes_of;

use crate::{
    RendererState, Vulkan,
    renderer::buffer::RenderBuffer,
    shaders::render_selection::{self, Selection, vertex_main::Vertex},
};

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Placement {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub struct SelectionPipeline {
    vulkan: Arc<Vulkan>,

    pub pipeline_layout: vk::PipelineLayout,
    pub shaders: Vec<vk::ShaderEXT>,
    pub stages: Vec<vk::ShaderStageFlags>,
}

impl SelectionPipeline {
    /// The colour of the selection shading.
    const COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
    /// The vertices to build the selection shading, counter-clockwise, triangle-strip.
    pub const VERTICIES: [Vertex; 10] = [
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft as u32,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft as u32,
            movable: vk::TRUE,
        },
        Vertex {
            position: [1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopRight as u32,
            movable: vk::FALSE,
        },
        Vertex {
            position: [1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopRight as u32,
            movable: vk::TRUE,
        },
        Vertex {
            position: [1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomRight as u32,
            movable: vk::FALSE,
        },
        Vertex {
            position: [1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomRight as u32,
            movable: vk::TRUE,
        },
        Vertex {
            position: [-1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomLeft as u32,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomLeft as u32,
            movable: vk::TRUE,
        },
        //
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft as u32,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft as u32,
            movable: vk::TRUE,
        },
    ];

    /// Create a new instance of the pipeline.
    pub unsafe fn new(vulkan: Arc<Vulkan>) -> LabelledVkResult<Self> {
        let pipeline_layout = {
            let push_range = Selection::push_constant_range();

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .push_constant_ranges(slice::from_ref(&push_range));

            let layout = unsafe {
                vulkan
                    .device()
                    .create_pipeline_layout(&create_info, VK_GLOBAL_ALLOCATOR.as_deref())
                    .map_err(|e| VkError::new(e, "vkCreatePiplineLayout"))?
            };

            unsafe { try_name(vulkan.as_ref(), layout, "SelectionPipeline Pipeline Layout") };

            layout
        };

        let (shaders, stages) = {
            let push_range = Selection::push_constant_range();

            let vertex_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_selection::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_selection::vertex_main::STAGE)
                .name(render_selection::vertex_main::ENTRY_POINT)
                .push_constant_ranges(slice::from_ref(&push_range));

            let fragment_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_selection::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_selection::fragment_main::STAGE)
                .name(render_selection::fragment_main::ENTRY_POINT)
                .push_constant_ranges(slice::from_ref(&push_range));

            let mut create_infos = [vertex_create_info, fragment_create_info];

            let stages: Vec<_> = create_infos.iter().map(|info| info.stage).collect();

            let shaders = unsafe {
                link_shader_objects(vulkan.as_ref(), &mut create_infos, "RENDER SELECTION")
            }
            .map_err(|e| VkError::new(e, "vkCreateShadersEXT"))?;

            (shaders, stages)
        };

        Ok(Self {
            vulkan,
            pipeline_layout,
            shaders,
            stages,
        })
    }

    pub unsafe fn cmd_draw(
        &self,
        command_buffer: vk::CommandBuffer,
        swapchain: &Swapchain,
        render_buffer: &RenderBuffer,
        state: RendererState,
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
                slice::from_ref(&render_buffer.selection_offset),
            );
        }

        // Push constants
        unsafe {
            let push_constants = Selection {
                start: swapchain.screen_to_vulkan_space(state.selection[0]),
                end: swapchain.screen_to_vulkan_space(state.selection[1]),
            };

            self.vulkan.device().cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                Selection::STAGES,
                0,
                bytes_of(&push_constants),
            );
        }

        // Draw
        unsafe {
            self.vulkan
                .device()
                .cmd_draw(command_buffer, Self::VERTICIES.len() as u32, 1, 0, 0);
        }
    }

    pub unsafe fn cmd_set_state(&self, command_buffer: vk::CommandBuffer) {
        let shader_device: &ext::shader_object::Device = unsafe { self.vulkan.context() };

        unsafe {
            shader_device.cmd_set_vertex_input(
                command_buffer,
                &render_selection::vertex_main::vertex_binding_descriptions_2_ext(),
                &render_selection::vertex_main::vertex_attribute_descriptions_2_ext(),
            );
        }

        unsafe {
            shader_device
                .cmd_set_primitive_topology(command_buffer, vk::PrimitiveTopology::TRIANGLE_STRIP);
            shader_device.cmd_set_polygon_mode(command_buffer, vk::PolygonMode::FILL);
        }
    }
}

impl Drop for SelectionPipeline {
    fn drop(&mut self) {
        unsafe {
            let shader_device: &ext::shader_object::Device = self.vulkan.context();

            self.shaders.iter().for_each(|shader| {
                shader_device.destroy_shader(*shader, VK_GLOBAL_ALLOCATOR.as_deref())
            });

            self.vulkan
                .device()
                .destroy_pipeline_layout(self.pipeline_layout, VK_GLOBAL_ALLOCATOR.as_deref());
        }
    }
}
