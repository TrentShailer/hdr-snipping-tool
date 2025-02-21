use ash::vk;
use ash_helper::{
    LabelledVkResult, VkError, VulkanContext, create_shader_module_from_spv, try_name,
};
use bytemuck::{Pod, Zeroable};
use core::{mem::offset_of, slice};

use crate::Vulkan;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub index: u32,
}

/// A line defined by a start and end position in Vulkan coordinates.
#[repr(C)]
#[derive(Default, Zeroable, Pod, Clone, Copy)]
pub struct Line {
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub colour: [f32; 4],
}

impl Line {
    pub fn start(mut self, start: [f32; 2]) -> Self {
        self.start[0] = start[0];
        self.start[1] = start[1];
        self
    }

    pub fn end(mut self, end: [f32; 2]) -> Self {
        self.end[0] = end[0];
        self.end[1] = end[1];
        self
    }

    pub fn colour(mut self, colour: [f32; 4]) -> Self {
        self.colour = colour;
        self
    }
}

#[derive(Clone)]
pub struct LinePipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub shader: vk::ShaderModule,
}

impl LinePipeline {
    /// The verticies to build the selection shading, line list.
    pub const VERTICIES: [Vertex; 2] = [Vertex { index: 0 }, Vertex { index: 1 }];

    /// Create a new instance of the pipeline.
    pub unsafe fn new(
        vulkan: &Vulkan,
        surface_format: vk::SurfaceFormatKHR,
    ) -> LabelledVkResult<Self> {
        let shader = unsafe {
            create_shader_module_from_spv(
                vulkan,
                include_bytes!("../../_shaders/spv/render_line.spv"),
            )?
        };

        let layout = {
            let push_constant_range = vk::PushConstantRange::default()
                .offset(0)
                .size(size_of::<Line>() as u32)
                .stage_flags(vk::ShaderStageFlags::VERTEX);

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .push_constant_ranges(slice::from_ref(&push_constant_range));

            unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePipelineLayout"))?
        };

        let pipeline = unsafe { Self::create_pipeline(vulkan, surface_format, layout, shader)? };

        Ok(Self {
            pipeline,
            layout,
            shader,
        })
    }

    /// Create the `vk::Pipeline` object.
    ///
    /// This is used on first creation and during recreation.
    pub unsafe fn create_pipeline(
        vulkan: &Vulkan,
        surface_format: vk::SurfaceFormatKHR,
        layout: vk::PipelineLayout,
        shader: vk::ShaderModule,
    ) -> LabelledVkResult<vk::Pipeline> {
        let bindings = [
            // Vertex
            vk::VertexInputBindingDescription::default()
                .binding(0)
                .stride(size_of::<Vertex>() as u32)
                .input_rate(vk::VertexInputRate::VERTEX),
        ];

        let attributes = [
            // Vertex
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32_UINT)
                .offset(offset_of!(Vertex, index) as u32),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&attributes)
            .vertex_binding_descriptions(&bindings);

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader)
                .name(c"main"),
        ];

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterisation_state = vk::PipelineRasterizationStateCreateInfo::default()
            .front_face(vk::FrontFace::CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::LINE);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let attachment_blend = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(slice::from_ref(&attachment_blend));

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::LINE_LIST);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
            vk::DynamicState::LINE_WIDTH,
            vk::DynamicState::VIEWPORT,
            vk::DynamicState::SCISSOR,
        ]);

        let mut rendering_create_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(slice::from_ref(&surface_format.format));

        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .input_assembly_state(&input_assembly_state)
            .rasterization_state(&rasterisation_state)
            .vertex_input_state(&vertex_input_info)
            .multisample_state(&multisample_state)
            .color_blend_state(&blend_state)
            .viewport_state(&viewport_state)
            .dynamic_state(&dynamic_state)
            .stages(&shader_stages)
            .layout(layout)
            .push_next(&mut rendering_create_info);

        let pipeline = unsafe {
            vulkan
                .device()
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    slice::from_ref(&create_info),
                    None,
                )
                .map_err(|(_, e)| VkError::new(e, "vkCreateGraphicsPipelines"))?[0]
        };

        unsafe { try_name(vulkan, pipeline, "Line Pipeline") };

        Ok(pipeline)
    }

    /// Recreate the pipeline when required.
    pub unsafe fn recreate(
        &mut self,
        vulkan: &Vulkan,
        surface_format: vk::SurfaceFormatKHR,
    ) -> LabelledVkResult<()> {
        unsafe { vulkan.device().destroy_pipeline(self.pipeline, None) };

        let pipeline =
            unsafe { Self::create_pipeline(vulkan, surface_format, self.layout, self.shader)? };

        self.pipeline = pipeline;

        Ok(())
    }

    /// Destroy the Vulkan resources.
    pub unsafe fn destroy(&self, vulkan: &Vulkan) {
        unsafe {
            vulkan.device().destroy_pipeline(self.pipeline, None);
            vulkan.device().destroy_pipeline_layout(self.layout, None);
            vulkan.device().destroy_shader_module(self.shader, None);
        }
    }
}
