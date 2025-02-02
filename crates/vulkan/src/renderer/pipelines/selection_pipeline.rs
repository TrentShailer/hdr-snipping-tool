use core::{mem::offset_of, slice};

use ash::vk;
use ash_helper::{
    create_shader_module_from_spv, try_name, LabelledVkResult, VkError, VulkanContext,
};
use bytemuck::{Pod, Zeroable};

use crate::Vulkan;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Placement {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
    pub colour: [f32; 4],
    pub placement: Placement,
    pub movable: vk::Bool32,
}

#[repr(C)]
#[derive(Default, Zeroable, Pod, Clone, Copy)]
pub struct Selection {
    pub start: [f32; 2],
    pub end: [f32; 2],
}

#[derive(Clone)]
pub struct SelectionPipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub shader: vk::ShaderModule,
}

impl SelectionPipeline {
    /// The colour of the selection shading.
    const COLOUR: [f32; 4] = [0.0, 0.0, 0.0, 0.5];
    /// The verticies to build the selection shading, counter-clockwise, triangle-strip.
    pub const VERTICIES: [Vertex; 10] = [
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft,
            movable: vk::TRUE,
        },
        Vertex {
            position: [1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopRight,
            movable: vk::FALSE,
        },
        Vertex {
            position: [1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopRight,
            movable: vk::TRUE,
        },
        Vertex {
            position: [1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomRight,
            movable: vk::FALSE,
        },
        Vertex {
            position: [1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomRight,
            movable: vk::TRUE,
        },
        Vertex {
            position: [-1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomLeft,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, 1.0],
            colour: Self::COLOUR,
            placement: Placement::BottomLeft,
            movable: vk::TRUE,
        },
        //
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft,
            movable: vk::FALSE,
        },
        Vertex {
            position: [-1.0, -1.0],
            colour: Self::COLOUR,
            placement: Placement::TopLeft,
            movable: vk::TRUE,
        },
    ];

    /// Create a new instance of the pipeline.
    pub unsafe fn new(
        vulkan: &Vulkan,
        surface_format: vk::SurfaceFormatKHR,
    ) -> LabelledVkResult<Self> {
        let shader = unsafe {
            create_shader_module_from_spv(
                vulkan,
                include_bytes!("../../_shaders/spv/render_selection.spv"),
            )?
        };

        let layout = {
            let push_constant_range = vk::PushConstantRange::default()
                .offset(0)
                .size(size_of::<Selection>() as u32)
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
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32),
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex, colour) as u32),
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32_UINT)
                .offset(offset_of!(Vertex, placement) as u32),
            vk::VertexInputAttributeDescription::default()
                .location(3)
                .binding(0)
                .format(vk::Format::R32_UINT)
                .offset(offset_of!(Vertex, movable) as u32),
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
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0);

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
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP);

        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);

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

        unsafe { try_name(vulkan, pipeline, "Selection Pipeline") };

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
        vulkan.device().destroy_pipeline(self.pipeline, None);
        vulkan.device().destroy_pipeline_layout(self.layout, None);
        vulkan.device().destroy_shader_module(self.shader, None);
    }
}
