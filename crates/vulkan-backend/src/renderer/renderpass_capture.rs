pub mod load_image;
pub mod render;

use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::PersistentDescriptorSet,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::VertexDefinition,
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    Validated, ValidationError, VulkanError,
};

use crate::{texture::Texture, VulkanInstance};

use super::vertex::Vertex;

const PLANE_VERTICIES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, -1.0],
        uv: [0.0, 0.0],
        color: 0,
    }, // TL
    Vertex {
        position: [1.0, -1.0],
        uv: [1.0, 0.0],
        color: 0,
    }, // TR
    Vertex {
        position: [1.0, 1.0],
        uv: [1.0, 1.0],
        color: 0,
    }, // BR
    Vertex {
        position: [-1.0, 1.0],
        uv: [0.0, 1.0],
        color: 0,
    }, // BL
];

const PLANE_INDICIES: [u32; 6] = [0, 1, 2, 2, 3, 0];

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        bytes: "shaders/vertex/capture.spv"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        bytes: "shaders/fragment/capture.spv"
    }
}

// In charge of rendering the frame capture
pub struct RenderpassCapture {
    pub capture: Option<Texture>,
    pub capture_ds: Option<Arc<PersistentDescriptorSet>>,
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
}

impl RenderpassCapture {
    pub fn new(instance: &VulkanInstance, subpass: Subpass) -> Result<Self, Error> {
        let vertex_buffer = Buffer::from_iter(
            instance.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            PLANE_VERTICIES,
        )?;

        let index_buffer = Buffer::from_iter(
            instance.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            PLANE_INDICIES,
        )?;

        let pipeline = {
            let vs = vertex_shader::load(instance.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap();
            let fs = fragment_shader::load(instance.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap();

            let vertex_input_state =
                <Vertex as vulkano::pipeline::graphics::vertex_input::Vertex>::per_vertex()
                    .definition(&vs.info().input_interface)
                    .map_err(Error::VertexDefinition)?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];

            let layout = PipelineLayout::new(
                instance.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(instance.device.clone())
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
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            };

            GraphicsPipeline::new(instance.device.clone(), None, graphics_pipeline_create_info)
                .map_err(Error::CreateGraphicsPipeline)?
        };

        Ok(Self {
            capture: None,
            capture_ds: None,
            vertex_buffer,
            index_buffer,
            pipeline,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to allocate buffer:\n{0:?}")]
    BufferAllocation(#[from] Validated<AllocateBufferError>),

    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to get vertex definition:\n{0}")]
    VertexDefinition(#[source] Box<ValidationError>),

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
