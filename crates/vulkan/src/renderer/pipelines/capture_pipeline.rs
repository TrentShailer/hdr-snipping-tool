use alloc::sync::Arc;
use core::slice;

use ash::{ext, khr, vk};
use ash_helper::{
    Context, LabelledVkResult, VkError, VulkanContext, link_shader_objects, try_name, try_name_all,
};
use bytemuck::bytes_of;

use crate::{
    RendererState, Vulkan,
    renderer::buffer::RenderBuffer,
    shaders::render_capture::{self, PushConstants, vertex_main::Vertex},
};

#[derive(Clone)]
pub struct CapturePipeline {
    vulkan: Arc<Vulkan>,

    pub descriptor_layouts: Vec<vk::DescriptorSetLayout>,
    pub pipeline_layout: vk::PipelineLayout,
    pub shaders: Vec<vk::ShaderEXT>,
    pub stages: Vec<vk::ShaderStageFlags>,

    pub sampler: vk::Sampler,
}

impl CapturePipeline {
    /// The verticies to build the selection shading, counter clockwise, triangle strip.
    pub const VERTICIES: [Vertex; 4] = [
        Vertex {
            position: [1.0, -1.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-1.0, -1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [1.0, 1.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [-1.0, 1.0],
            uv: [0.0, 1.0],
        },
    ];

    /// Create a new instance of the pipeline.
    pub unsafe fn new(vulkan: Arc<Vulkan>) -> LabelledVkResult<Self> {
        let sampler = {
            let create_info = vk::SamplerCreateInfo::default();

            let sampler = unsafe { vulkan.device().create_sampler(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateSampler"))?;

            unsafe { try_name(vulkan.as_ref(), sampler, "Capture Sampler") };

            sampler
        };

        let descriptor_layouts = {
            let layouts = unsafe {
                render_capture::set_layouts(
                    vulkan.device(),
                    vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
                )
                .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?
            };

            unsafe {
                try_name_all(
                    vulkan.as_ref(),
                    &layouts,
                    "Render Capture Descriptor Layout",
                )
            };

            layouts
        };

        let pipeline_layout = {
            let push_range = PushConstants::push_constant_range();

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(slice::from_ref(&push_range));

            let layout = unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePiplineLayout"))?;

            unsafe { try_name(vulkan.as_ref(), layout, "Render Capture Pipeline Layout") };

            layout
        };

        let (shaders, stages) = {
            let push_range = PushConstants::push_constant_range();

            let vertex_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_capture::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_capture::vertex_main::STAGE)
                .name(render_capture::vertex_main::ENTRY_POINT)
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(slice::from_ref(&push_range));

            let fragment_create_info = vk::ShaderCreateInfoEXT::default()
                .code(render_capture::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(render_capture::fragment_main::STAGE)
                .name(render_capture::fragment_main::ENTRY_POINT)
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(slice::from_ref(&push_range));

            let mut create_infos = [vertex_create_info, fragment_create_info];
            let stages: Vec<_> = create_infos.iter().map(|info| info.stage).collect();

            let shaders = unsafe {
                link_shader_objects(vulkan.as_ref(), &mut create_infos, "RENDER CAPTURE")
                    .map_err(|e| VkError::new(e, "vkCreateShadersEXT"))?
            };

            (shaders, stages)
        };

        Ok(Self {
            vulkan,
            descriptor_layouts,
            pipeline_layout,
            shaders,
            stages,
            sampler,
        })
    }

    pub unsafe fn cmd_draw(
        &self,
        command_buffer: vk::CommandBuffer,
        surface_format: vk::SurfaceFormatKHR,
        render_buffer: &RenderBuffer,
        state: RendererState,
    ) {
        let Some(capture) = state.capture else { return };

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
                slice::from_ref(&render_buffer.capture_offset),
            );
        }

        // Push descriptors
        {
            let image_info = vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(capture.view)
                .sampler(self.sampler);

            let descriptor_write = vk::WriteDescriptorSet::default()
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&image_info));

            unsafe {
                let device: &khr::push_descriptor::Device = self.vulkan.context();
                device.cmd_push_descriptor_set(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    slice::from_ref(&descriptor_write),
                );
            }
        }

        // Push constants
        unsafe {
            let present_srgb =
                if surface_format.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT {
                    vk::FALSE
                } else {
                    vk::TRUE
                };

            let push_constants = PushConstants {
                max_brightness: state.max_brightness,
                whitepoint: state.whitepoint,
                present_srgb,
            };

            self.vulkan.device().cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                PushConstants::STAGES,
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
                &render_capture::vertex_main::vertex_binding_descriptions_2_ext(),
                &render_capture::vertex_main::vertex_attribute_descriptions_2_ext(),
            );
        }

        unsafe {
            shader_device
                .cmd_set_primitive_topology(command_buffer, vk::PrimitiveTopology::TRIANGLE_STRIP);
            shader_device.cmd_set_polygon_mode(command_buffer, vk::PolygonMode::FILL);
        }
    }
}

impl Drop for CapturePipeline {
    fn drop(&mut self) {
        unsafe {
            let shader_device: &ext::shader_object::Device = self.vulkan.context();

            self.shaders
                .iter()
                .for_each(|shader| shader_device.destroy_shader(*shader, None));

            self.vulkan
                .device()
                .destroy_pipeline_layout(self.pipeline_layout, None);

            self.descriptor_layouts.iter().for_each(|layout| {
                self.vulkan
                    .device()
                    .destroy_descriptor_set_layout(*layout, None);
            });

            self.vulkan.device().destroy_sampler(self.sampler, None);
        }
    }
}
