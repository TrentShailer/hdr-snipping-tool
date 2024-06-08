pub mod framebuffer;
pub mod render;
pub mod renderpass_border;
pub mod renderpass_capture;
pub mod renderpass_final;
pub mod renderpass_mouse;
pub mod renderpass_selection;
pub mod vertex;

use std::sync::Arc;

use framebuffer::window_size_dependent_setup;
use renderpass_border::RenderpassBorder;
use renderpass_capture::RenderpassCapture;
use renderpass_final::RenderpassFinal;
use renderpass_mouse::RenderpassMouse;
use renderpass_selection::RenderpassSelection;
use thiserror::Error;
use vulkano::{
    buffer::AllocateBufferError,
    format::Format,
    image::{view::ImageView, ImageUsage},
    ordered_passes_renderpass,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass, Subpass},
    swapchain::{Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::window::Window;

use crate::VulkanInstance;

pub struct Renderer {
    pub recreate_swapchain: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub attachments: Arc<[Arc<ImageView>; 4]>,
    pub render_pass: Arc<RenderPass>,
    pub viewport: Viewport,
    pub renderpass_capture: RenderpassCapture,
    pub renderpass_selection: RenderpassSelection,
    pub renderpass_border: RenderpassBorder,
    pub renderpass_mouse: RenderpassMouse,
    pub renderpass_final: RenderpassFinal,
}

impl Renderer {
    pub fn new(instance: &VulkanInstance, window: Arc<Window>) -> Result<Self, Error> {
        let (swapchain, images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can only pass
            // values that are allowed by the capabilities.
            let surface_capabilities = instance
                .device
                .physical_device()
                .surface_capabilities(&instance.surface, Default::default())
                .map_err(Error::GetSurfaceCapabilites)?;

            // Choosing the internal format that the images will have.
            let image_format = instance
                .device
                .physical_device()
                .surface_formats(&instance.surface, Default::default())
                .map_err(Error::GetSurfaceFormats)?[0]
                .0;

            let composite_alpha = surface_capabilities
                .supported_composite_alpha
                .into_iter()
                .next()
                .ok_or(Error::CompositeAlpha)?;

            Swapchain::new(
                instance.device.clone(),
                instance.surface.clone(),
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

        let render_pass = ordered_passes_renderpass!(instance.device.clone(),
            attachments: {
                final_color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                capture_out: {
                    format: Format::R8G8B8A8_SRGB,
                    samples: 1,
                    load_op: DontCare,
                    store_op: DontCare,
                },
                selection_out: {
                    format: Format::R8G8B8A8_SRGB,
                    samples: 1,
                    load_op: DontCare,
                    store_op: DontCare,
                },
                border_out: {
                    format: Format::R8G8B8A8_SRGB,
                    samples: 1,
                    load_op: DontCare,
                    store_op: DontCare,
                },
                mouse_out: {
                    format: Format::R8G8B8A8_SRGB, // TODO this could be more efficent
                    samples: 1,
                    load_op: DontCare,
                    store_op: DontCare,
                },
            },
            passes: [
                { // Capture Renderer Pass
                    color: [capture_out], // output
                    depth_stencil: {},
                    input: []
                },
                { // Selection Renderer Pass
                    color: [selection_out],
                    depth_stencil: {},
                    input: []
                },
                { // Border Renderer Pass
                    color: [border_out],
                    depth_stencil: {},
                    input: []
                },
                { // Mouse guides renderer pass
                    color: [mouse_out],
                    depth_stencil: {},
                    input: []
                },
                { // Final pass
                    color: [final_color],
                    depth_stencil: {},
                    input: [capture_out, selection_out, mouse_out]
                }
            ]
        )
        .map_err(Error::CreateRenderpass)?;

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let (framebuffers, attachments) =
            window_size_dependent_setup(&instance, &images, render_pass.clone(), &mut viewport)?;

        let capture_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let renderpass_capture = RenderpassCapture::new(&instance, capture_pass)?;

        let selection_pass = Subpass::from(render_pass.clone(), 1).unwrap();
        let renderpass_selection = RenderpassSelection::new(&instance, selection_pass)?;

        let border_pass = Subpass::from(render_pass.clone(), 2).unwrap();
        let renderpass_border = RenderpassBorder::new(&instance, border_pass)?;

        let mouse_pass = Subpass::from(render_pass.clone(), 3).unwrap();
        let renderpass_mouse = RenderpassMouse::new(&instance, mouse_pass)?;

        let final_pass = Subpass::from(render_pass.clone(), 4).unwrap();
        let renderpass_final = RenderpassFinal::new(&instance, final_pass, attachments.clone())?;

        Ok(Self {
            attachments,
            framebuffers,
            render_pass,
            swapchain,
            viewport,
            renderpass_capture,
            renderpass_selection,
            renderpass_border,
            renderpass_mouse,
            renderpass_final,
            previous_frame_end: Some(sync::now(instance.device.clone()).boxed()),
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

    #[error("Failed to create graphics pipeline:\n{0:?}")]
    CreateGraphicsPipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create framebuffers:\n{0}")]
    Framebuffer(#[from] framebuffer::Error),

    #[error("Into Pipeline Layout Info Error:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },

    #[error("Failed to create capture pass:\n{0}")]
    CreateCapturePass(#[from] renderpass_capture::Error),

    #[error("Failed to create selection pass:\n{0}")]
    CreateSelectionPass(#[from] renderpass_selection::Error),

    #[error("Failed to create border pass:\n{0}")]
    CreateBorderPass(#[from] renderpass_border::Error),

    #[error("Failed to create mouse pass:\n{0}")]
    CreateMousePass(#[from] renderpass_mouse::Error),

    #[error("Failed to create final pass:\n{0}")]
    CreateFinalPass(#[from] renderpass_final::Error),
}
