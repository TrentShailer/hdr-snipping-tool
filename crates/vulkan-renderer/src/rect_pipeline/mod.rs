pub mod rect;
pub mod vertex;

use std::sync::Arc;

use thiserror::Error;
use vertex::Vertex;
use vulkan_instance::VulkanInstance;
use vulkano::{
    pipeline::{
        graphics::{
            color_blend::AttachmentBlend,
            vertex_input::{Vertex as VkVertex, VertexDefinition},
        },
        GraphicsPipeline, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    Validated, ValidationError, VulkanError,
};

pub mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        bytes: "src/rect_pipeline/shaders/vertex.spv"
    }
}

pub mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        bytes: "src/rect_pipeline/shaders/fragment.spv"
    }
}

pub fn create_pipeline(
    vk: &VulkanInstance,
    subpass: Subpass,
) -> Result<Arc<GraphicsPipeline>, Error> {
    let vs = vertex_shader::load(vk.device.clone())
        .map_err(Error::LoadShader)?
        .entry_point("main")
        .unwrap();

    let fs = fragment_shader::load(vk.device.clone())
        .map_err(Error::LoadShader)?
        .entry_point("main")
        .unwrap();

    let vertex_input_state = Vertex::per_vertex()
        .definition(&vs.info().input_interface)
        .map_err(Error::VertexDefinition)?;

    let stages = vec![
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let pipeline = crate::graphics_pipeline::create_pipeline(
        &vk,
        subpass,
        vertex_input_state,
        stages,
        Some(AttachmentBlend::alpha()),
    )?;

    Ok(pipeline)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to get vertex definition:\n{0}")]
    VertexDefinition(#[source] Box<ValidationError>),

    #[error("Failed to create graphics pipeline:\n{0}")]
    GraphicsPipeline(#[from] crate::graphics_pipeline::Error),
}
