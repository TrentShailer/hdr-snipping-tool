use std::sync::Arc;

use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::BufferContents,
    pipeline::{
        graphics::{
            color_blend::AttachmentBlend,
            subpass::PipelineRenderingCreateInfo,
            vertex_input::{Vertex as VkVertex, VertexDefinition},
        },
        GraphicsPipeline, PipelineShaderStageCreateInfo,
    },
};

use super::Error;

#[derive(Clone, Copy, BufferContents, VkVertex)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],

    #[format(R8G8B8A8_UNORM)]
    pub color: [u8; 4],
}

#[derive(Clone, Copy, BufferContents, VkVertex)]
#[repr(C)]
pub struct InstanceData {
    #[format(R32G32_SFLOAT)]
    pub glyph_position: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub glyph_size: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub bitmap_size: [f32; 2],

    #[format(R32G32_SFLOAT)]
    pub uv_offset: [f32; 2],
}

pub mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        bytes: "src/shaders/text.vert.spv"
    }
}

pub mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        bytes: "src/shaders/text.frag.spv"
    }
}

pub fn create_pipeline(
    vk: &VulkanInstance,
    subpass: PipelineRenderingCreateInfo,
) -> Result<Arc<GraphicsPipeline>, Error> {
    let vs = vertex_shader::load(vk.device.clone())
        .map_err(Error::LoadShader)?
        .entry_point("main")
        .unwrap();

    let fs = fragment_shader::load(vk.device.clone())
        .map_err(Error::LoadShader)?
        .entry_point("main")
        .unwrap();

    let vertex_input_state = [Vertex::per_vertex(), InstanceData::per_instance()]
        .definition(&vs.info().input_interface)
        .map_err(Error::VertexDefinition)?;

    let stages = vec![
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let pipeline = super::create_pipeline(
        vk,
        subpass,
        vertex_input_state,
        stages,
        Some(AttachmentBlend::alpha()),
    )?;

    Ok(pipeline)
}
