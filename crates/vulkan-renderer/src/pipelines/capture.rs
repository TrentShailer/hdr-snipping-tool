use std::{io::Cursor, mem::offset_of};

use ash::{
    util::read_spv,
    vk::{
        BlendFactor, BlendOp, ColorComponentFlags, DescriptorSetLayout, DescriptorSetLayoutBinding,
        DescriptorSetLayoutCreateInfo, DescriptorType, Format, Pipeline,
        PipelineColorBlendAttachmentState, PipelineLayout, PipelineLayoutCreateInfo,
        PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo,
        PipelineVertexInputStateCreateInfo, PushConstantRange, ShaderModule,
        ShaderModuleCreateInfo, ShaderStageFlags, VertexInputAttributeDescription,
        VertexInputBindingDescription, VertexInputRate, Viewport,
    },
};
use vulkan_instance::VulkanInstance;

use super::Error;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

pub fn create_pipeline(
    vk: &VulkanInstance,
    pipeline_rendering_create_info: PipelineRenderingCreateInfo,
    viewport: Viewport,
) -> Result<
    (
        Pipeline,
        PipelineLayout,
        [ShaderModule; 2],
        [DescriptorSetLayout; 2],
    ),
    Error,
> {
    let vertex_input_binding_descriptions = [VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<Vertex>() as u32,
        input_rate: VertexInputRate::VERTEX,
    }];

    let vertex_attribute_descriptions = [
        VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(Vertex, position) as u32,
        },
        VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: Format::R32G32_SFLOAT,
            offset: offset_of!(Vertex, uv) as u32,
        },
    ];
    let vertex_input_state_info = PipelineVertexInputStateCreateInfo::default()
        .vertex_attribute_descriptions(&vertex_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_input_binding_descriptions);

    let color_blend_attachment_state = PipelineColorBlendAttachmentState {
        blend_enable: 1,
        src_color_blend_factor: BlendFactor::SRC_ALPHA,
        dst_color_blend_factor: BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: BlendOp::ADD,
        src_alpha_blend_factor: BlendFactor::SRC_ALPHA,
        dst_alpha_blend_factor: BlendFactor::ONE_MINUS_SRC_ALPHA,
        alpha_blend_op: BlendOp::ADD,
        color_write_mask: ColorComponentFlags::RGBA,
    };

    let (vs, vs_entry) = unsafe {
        let mut shader_file = Cursor::new(&include_bytes!("../shaders/capture.vert.spv")[..]);
        let shader_code = read_spv(&mut shader_file).map_err(Error::ReadShader)?;
        let shader_info = ShaderModuleCreateInfo::default().code(&shader_code);
        let shader_module = vk
            .device
            .create_shader_module(&shader_info, None)
            .map_err(|e| Error::Vulkan(e, "creating shader module"))?;
        let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");
        (shader_module, shader_entry_name)
    };
    let (fs, fs_entry) = unsafe {
        let mut shader_file = Cursor::new(&include_bytes!("../shaders/capture.frag.spv")[..]);
        let shader_code = read_spv(&mut shader_file).map_err(Error::ReadShader)?;
        let shader_info = ShaderModuleCreateInfo::default().code(&shader_code);
        let shader_module = vk
            .device
            .create_shader_module(&shader_info, None)
            .map_err(|e| Error::Vulkan(e, "creating shader module"))?;
        let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");
        (shader_module, shader_entry_name)
    };
    let shader_stage_create_infos = [
        PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::VERTEX)
            .module(vs)
            .name(vs_entry),
        PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::FRAGMENT)
            .module(fs)
            .name(fs_entry),
    ];

    let push_constant_ranges = [PushConstantRange {
        stage_flags: ShaderStageFlags::FRAGMENT,
        offset: 0,
        size: 4,
    }];

    let descriptor_layouts = unsafe {
        let sampler_layout = {
            let descriptor_layout_bindings = [DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: DescriptorType::SAMPLER,
                descriptor_count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }];

            let descriptor_info =
                DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_layout_bindings);

            vk.device
                .create_descriptor_set_layout(&descriptor_info, None)
                .map_err(|e| Error::Vulkan(e, "creating descriptor set layout"))?
        };

        let view_layout = {
            let descriptor_layout_bindings = [DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }];

            let descriptor_info =
                DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_layout_bindings);

            let descriptor_layout = vk
                .device
                .create_descriptor_set_layout(&descriptor_info, None)
                .map_err(|e| Error::Vulkan(e, "creating descriptor set layout"))?;

            descriptor_layout
        };

        [sampler_layout, view_layout]
    };

    let pipeline_layout_create_info = PipelineLayoutCreateInfo::default()
        .push_constant_ranges(&push_constant_ranges)
        .set_layouts(&descriptor_layouts);

    let (pipeline, pipeline_layout) = super::create_pipeline(
        vk,
        pipeline_rendering_create_info,
        pipeline_layout_create_info,
        vertex_input_state_info,
        &shader_stage_create_infos,
        color_blend_attachment_state,
        viewport,
    )?;

    Ok((pipeline, pipeline_layout, [vs, fs], descriptor_layouts))
}
