use core::{mem::offset_of, slice};

use ash::vk;
use ash_helper::{
    LabelledVkResult, VkError, VulkanContext, create_shader_module_from_spv, try_name,
};
use bytemuck::{Pod, Zeroable};

use crate::Vulkan;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Default, Zeroable, Pod, Clone, Copy)]
pub struct PushConstants {
    pub whitepoint: f32,
    pub max_brightness: f32,
}

#[derive(Clone)]
pub struct CapturePipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    pub shader: vk::ShaderModule,
    pub set_layout: vk::DescriptorSetLayout,
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
    pub unsafe fn new(
        vulkan: &Vulkan,
        surface_format: vk::SurfaceFormatKHR,
    ) -> LabelledVkResult<Self> {
        let shader = unsafe {
            create_shader_module_from_spv(
                vulkan,
                include_bytes!("../../_shaders/spv/render_capture.spv"),
            )?
        };

        let sampler = {
            let create_info = vk::SamplerCreateInfo::default();

            let sampler = unsafe { vulkan.device().create_sampler(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateSampler"))?;

            unsafe { try_name(vulkan, sampler, "Capture Sampler") };

            sampler
        };

        let set_layout = {
            let bindings = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX)
                .immutable_samplers(slice::from_ref(&sampler));

            let create_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(slice::from_ref(&bindings))
                .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR);

            let layout = unsafe {
                {
                    vulkan
                        .device()
                        .create_descriptor_set_layout(&create_info, None)
                }
                .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?
            };

            unsafe {
                try_name(vulkan, layout, "Capture Set Layout");
            }

            layout
        };

        let layout = {
            let push_constant_range = vk::PushConstantRange::default()
                .offset(0)
                .size(size_of::<PushConstants>() as u32)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX);

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(slice::from_ref(&set_layout))
                .push_constant_ranges(slice::from_ref(&push_constant_range));

            unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePipelineLayout"))?
        };

        let pipeline = unsafe { Self::create_pipeline(vulkan, surface_format, layout, shader)? };

        Ok(Self {
            pipeline,
            layout,
            shader,
            set_layout,
            sampler,
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
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&attributes)
            .vertex_binding_descriptions(&bindings);

        // Specialisation
        let specialisation_map = vk::SpecializationMapEntry::default()
            .constant_id(0)
            .offset(0)
            .size(4);

        let present_srgb =
            if surface_format.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT {
                vk::FALSE.to_ne_bytes()
            } else {
                vk::TRUE.to_ne_bytes()
            };

        let specialisation_info = vk::SpecializationInfo::default()
            .map_entries(slice::from_ref(&specialisation_map))
            .data(&present_srgb);

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader)
                .name(c"main")
                .specialization_info(&specialisation_info),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader)
                .name(c"main")
                .specialization_info(&specialisation_info),
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
        unsafe {
            vulkan.device().destroy_pipeline(self.pipeline, None);
            vulkan.device().destroy_pipeline_layout(self.layout, None);
            vulkan.device().destroy_shader_module(self.shader, None);
            vulkan
                .device()
                .destroy_descriptor_set_layout(self.set_layout, None);
            vulkan.device().destroy_sampler(self.sampler, None);
        }
    }
}
