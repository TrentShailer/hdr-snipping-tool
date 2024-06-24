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
            vertex_input::VertexInputState,
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    Validated, VulkanError,
};

pub fn create_pipeline(
    vk: &VulkanInstance,
    subpass: Subpass,
    vertex_input_state: VertexInputState,
    stages: Vec<PipelineShaderStageCreateInfo>,
    blend: Option<AttachmentBlend>,
) -> Result<Arc<GraphicsPipeline>, Error> {
    let layout = PipelineLayout::new(
        vk.device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(vk.device.clone())
            .map_err(|e| Error::CreatePipelineLayoutInfo {
                set_num: e.set_num,
                error: e.error,
            })?,
    )
    .map_err(Error::CreatePipelineLayout)?;

    let graphics_pipeline_create_info = GraphicsPipelineCreateInfo {
        stages: stages.into_iter().collect(),
        vertex_input_state: Some(vertex_input_state),
        input_assembly_state: Some(InputAssemblyState::default()), // Triangle list
        viewport_state: Some(ViewportState::default()),
        rasterization_state: Some(RasterizationState::default()),
        multisample_state: Some(MultisampleState::default()),
        color_blend_state: Some(ColorBlendState::with_attachment_states(
            subpass.num_color_attachments(),
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
    #[error("Failed to create pipline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create graphics pipeline:\n{0:?}")]
    CreateGraphicsPipeline(#[source] Validated<VulkanError>),

    #[error("Into Pipeline Layout Info Error:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },
}
