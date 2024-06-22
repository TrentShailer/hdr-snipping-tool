pub mod load_texture;
pub mod render;
pub mod window_size_dependent_setup;

use std::sync::Arc;

use window_size_dependent_setup::window_size_dependent_setup;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage},
    image::ImageUsage,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::VertexDefinition,
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
    single_pass_renderpass,
    swapchain::{Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::window::Window;

use crate::{
    plane::{PLANE_INDICIES, PLANE_VERTICIES},
    vertex::Vertex,
    Renderer,
};

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        bytes: "src/shaders/vertex.spv"
    }
}

pub mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        bytes: "src/shaders/fragment.spv"
    }
}

impl Renderer {
    pub fn new(vk: &VulkanInstance, window: Arc<Window>) -> Result<Self, Error> {
        let (swapchain, images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can only pass
            // values that are allowed by the capabilities.
            let surface_capabilities = vk
                .device
                .physical_device()
                .surface_capabilities(&vk.surface, Default::default())
                .map_err(Error::GetSurfaceCapabilites)?;

            // Choosing the internal format that the images will have.
            let image_format = vk
                .device
                .physical_device()
                .surface_formats(&vk.surface, Default::default())
                .map_err(Error::GetSurfaceFormats)?[0]
                .0;

            let composite_alpha = surface_capabilities
                .supported_composite_alpha
                .into_iter()
                .next()
                .ok_or(Error::CompositeAlpha)?;

            Swapchain::new(
                vk.device.clone(),
                vk.surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .map_err(Error::CreateSwapchain)?
        };

        let render_pass = single_pass_renderpass!(vk.device.clone(),
            attachments: {
                output_color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                }
            },
            pass: {
                color: [output_color],
                depth_stencil: {},
            },
        )
        .map_err(Error::CreateRenderpass)?;

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let framebuffers =
            window_size_dependent_setup(&images, render_pass.clone(), &mut viewport)?;

        let pipeline = {
            let vs = vertex_shader::load(vk.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap();
            let fs = fragment_shader::load(vk.device.clone())
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
                vk.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(vk.device.clone())
                    .map_err(|e| Error::CreatePipelineLayoutInfo {
                        set_num: e.set_num,
                        error: e.error,
                    })?,
            )
            .map_err(Error::CreatePipelineLayout)?;

            let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

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

            GraphicsPipeline::new(vk.device.clone(), None, graphics_pipeline_create_info)
                .map_err(Error::CreateGraphicsPipeline)?
        };

        let vertex_buffer = Buffer::from_iter(
            vk.allocators.memory.clone(),
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
            vk.allocators.memory.clone(),
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

        Ok(Self {
            framebuffers,
            viewport,
            swapchain,
            vertex_buffer,
            index_buffer,
            pipeline,
            render_pass,
            texture: None,
            texture_ds: None,
            previous_frame_end: Some(sync::now(vk.device.clone()).boxed()),
            recreate_swapchain: false,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get surface capabilites:\n{0:?}")]
    GetSurfaceCapabilites(#[source] Validated<VulkanError>),

    #[error("Failed to get surface formats:\n{0:?}")]
    GetSurfaceFormats(#[source] Validated<VulkanError>),

    #[error("Composite alpha is not supported")]
    CompositeAlpha,

    #[error("Failed to create swapchain:\n{0:?}")]
    CreateSwapchain(#[source] Validated<VulkanError>),

    #[error("Failed to allocate buffer:\n{0:?}")]
    BufferAllocation(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create renderpass:\n{0:?}")]
    CreateRenderpass(#[source] Validated<VulkanError>),

    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to get vertex definition:\n{0}")]
    VertexDefinition(#[source] Box<ValidationError>),

    #[error("Failed to create pipline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    CreateDescriptorSet(#[source] Validated<VulkanError>),

    #[error("Failed to create graphics pipeline:\n{0:?}")]
    CreateGraphicsPipeline(#[source] Validated<VulkanError>),

    #[error("Failed to perform Window Size Dependent Setup:\n{0}")]
    WindowSizeDependentSetup(#[from] window_size_dependent_setup::Error),

    #[error("Into Pipeline Layout Info Error:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },
}
