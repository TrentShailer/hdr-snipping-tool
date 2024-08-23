use std::mem::{offset_of, size_of};

use ash::vk::{
    BlendFactor, BlendOp, ColorComponentFlags, Format, Pipeline, PipelineColorBlendAttachmentState,
    PipelineLayout, PipelineLayoutCreateInfo, PipelineRenderingCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange,
    ShaderModule, ShaderStageFlags, VertexInputAttributeDescription, VertexInputBindingDescription,
    VertexInputRate, Viewport,
};
use bytemuck::{Pod, Zeroable};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [u8; 4],
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstants {
    pub base_position: [f32; 2],
    pub base_size: [f32; 2],
    pub target_position: [f32; 2],
    pub target_size: [f32; 2],
    pub line_size: [f32; 2],
}

#[instrument("boder::create_pipeline", skip_all, err)]
pub fn create_pipeline(
    vk: &VulkanInstance,
    pipeline_rendering_create_info: PipelineRenderingCreateInfo,
    viewport: Viewport,
) -> Result<(Pipeline, PipelineLayout, [ShaderModule; 2]), VulkanError> {
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
            format: Format::R8G8B8A8_UNORM,
            offset: offset_of!(Vertex, color) as u32,
        },
        VertexInputAttributeDescription {
            location: 2,
            binding: 0,
            format: Format::R32_UINT,
            offset: offset_of!(Vertex, flags) as u32,
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
        let shader_module =
            vk.create_shader_module(include_bytes!("../shaders/border.vert.spv"))?;
        let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");
        (shader_module, shader_entry_name)
    };
    let (fs, fs_entry) = unsafe {
        let shader_module =
            vk.create_shader_module(include_bytes!("../shaders/border.frag.spv"))?;
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
        stage_flags: ShaderStageFlags::VERTEX,
        offset: 0,
        size: size_of::<PushConstants>() as u32,
    }];

    let pipeline_layout_create_info =
        PipelineLayoutCreateInfo::default().push_constant_ranges(&push_constant_ranges);

    let (pipeline, pipeline_layout) = super::create_pipeline(
        vk,
        pipeline_rendering_create_info,
        pipeline_layout_create_info,
        vertex_input_state_info,
        &shader_stage_create_infos,
        color_blend_attachment_state,
        viewport,
    )?;

    Ok((pipeline, pipeline_layout, [vs, fs]))
}
