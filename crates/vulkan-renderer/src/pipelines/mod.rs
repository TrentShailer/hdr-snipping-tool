pub mod border;
pub mod capture;
pub mod mouse_guides;
pub mod selection_shading;
pub mod vertex_index_buffer;

use ash::vk::{
    DynamicState, FrontFace, GraphicsPipelineCreateInfo, LogicOp, Pipeline, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, RenderPass, SampleCountFlags,
    Viewport,
};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

pub const MAIN_CSTR: &std::ffi::CStr =
    unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0") };

/// Helper function to create a basic graphics pipeline with given inputs.
#[instrument(skip_all, err)]
pub fn create_pipeline(
    vk: &VulkanInstance,
    pipeline_rendering_create_info: PipelineRenderingCreateInfo,
    pipeline_layout: PipelineLayout,
    vertex_input_state: PipelineVertexInputStateCreateInfo,
    stages: &[PipelineShaderStageCreateInfo],
    blend: PipelineColorBlendAttachmentState,
    viewport: Viewport,
) -> Result<Pipeline, VulkanError> {
    let mut pipeline_rendering_create_info = pipeline_rendering_create_info;

    let input_assembly_state =
        PipelineInputAssemblyStateCreateInfo::default().topology(PrimitiveTopology::TRIANGLE_LIST);

    let viewports = [viewport];
    let viewport_state = PipelineViewportStateCreateInfo::default().viewports(&viewports);

    let rasterization_state = PipelineRasterizationStateCreateInfo::default()
        .front_face(FrontFace::CLOCKWISE)
        .line_width(1.0)
        .polygon_mode(PolygonMode::FILL);

    let multisample_state = PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(SampleCountFlags::TYPE_1);

    let color_blend_attachment_states = [blend];
    let color_blend_state = PipelineColorBlendStateCreateInfo::default()
        .logic_op(LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);

    let dynamic_states = [DynamicState::VIEWPORT, DynamicState::SCISSOR_WITH_COUNT];
    let dynamic_state = PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let graphics_pipeline_create_info = GraphicsPipelineCreateInfo::default()
        .stages(stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state)
        .layout(pipeline_layout)
        .render_pass(RenderPass::null())
        .push_next(&mut pipeline_rendering_create_info);

    let pipelines = unsafe {
        vk.device
            .create_graphics_pipelines(
                PipelineCache::null(),
                &[graphics_pipeline_create_info],
                None,
            )
            .map_err(|(_, e)| VulkanError::VkResult(e, "creating graphics pipline"))?
    };

    let pipeline = pipelines[0];

    Ok(pipeline)
}
