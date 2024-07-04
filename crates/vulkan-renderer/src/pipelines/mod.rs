pub mod border;
pub mod capture;
pub mod mouse_guides;
pub mod rect;
pub mod selection_shading;
pub mod text;

use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    pipeline::{
        graphics::{
            color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            subpass::PipelineRenderingCreateInfo,
            vertex_input::VertexInputState,
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::{IntoPipelineLayoutCreateInfoError, PipelineDescriptorSetLayoutCreateInfo},
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    Validated, ValidationError, VulkanError,
};

/// Helper function to create a basic graphics pipeline with given inputs.
pub fn create_pipeline(
    vk: &VulkanInstance,
    subpass: PipelineRenderingCreateInfo,
    vertex_input_state: VertexInputState,
    stages: Vec<PipelineShaderStageCreateInfo>,
    blend: Option<AttachmentBlend>,
) -> Result<Arc<GraphicsPipeline>, Error> {
    let pipeline_ds_layout_create_info =
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(vk.device.clone())?;

    let layout = PipelineLayout::new(vk.device.clone(), pipeline_ds_layout_create_info)
        .map_err(Error::CreatePipelineLayout)?;

    let graphics_pipeline_create_info = GraphicsPipelineCreateInfo {
        stages: stages.into_iter().collect(),
        vertex_input_state: Some(vertex_input_state),
        input_assembly_state: Some(InputAssemblyState::default()), // Triangle list
        viewport_state: Some(ViewportState::default()),
        rasterization_state: Some(RasterizationState::default()),
        multisample_state: Some(MultisampleState::default()),
        color_blend_state: Some(ColorBlendState::with_attachment_states(
            subpass.color_attachment_formats.len() as u32,
            ColorBlendAttachmentState {
                blend,
                ..Default::default()
            },
        )),
        dynamic_state: [DynamicState::Viewport].into_iter().collect(),
        subpass: Some(subpass.into()),
        ..GraphicsPipelineCreateInfo::layout(layout)
    };

    let pipeline = GraphicsPipeline::new(vk.device.clone(), None, graphics_pipeline_create_info)
        .map_err(Error::CreateGraphicsPipeline)?;

    Ok(pipeline)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to get vertex definition:\n{0}")]
    VertexDefinition(#[source] Box<ValidationError>),

    #[error("Failed to create pipline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create graphics pipeline:\n{0:?}")]
    CreateGraphicsPipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create pipeline layout info:\n{0:?}")]
    CreatePipelineLayoutInfo(#[from] IntoPipelineLayoutCreateInfoError),
}
