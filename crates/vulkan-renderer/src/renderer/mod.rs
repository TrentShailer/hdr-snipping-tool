pub mod render;
pub mod units;
pub mod window_size_dependent_setup;

use std::sync::Arc;

use crate::{
    border_pipeline::{self, border::Border},
    capture_pipeline::{self, capture::CaptureObject},
    mouse_pipeline::{self, mouse::Mouse},
    parameters_pipeline::{self, parameters::Parameters},
    rect_pipeline::{self, rect::Rect},
    selection_pipeline::{self, selection::Selection},
};
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    image::ImageUsage,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass, Subpass},
    single_pass_renderpass,
    swapchain::{Swapchain, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    Validated, VulkanError,
};
use window_size_dependent_setup::window_size_dependent_setup;
use winit::window::Window;

pub struct Renderer {
    pub recreate_swapchain: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub render_pass: Arc<RenderPass>,
    pub viewport: Viewport,
    //
    pub capture: CaptureObject,
    pub selection: Selection,
    pub selection_border: Border,
    pub mouse: Mouse,
    pub parameters: Parameters,
    pub text_rect: Rect,
    pub text_border: Border,
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
            log::debug!("Image format: {:?}", image_format);

            let composite_alpha = surface_capabilities
                .supported_composite_alpha
                .into_iter()
                .next()
                .ok_or(Error::CompositeAlpha)?;

            let present_modes = vk
                .device
                .physical_device()
                .surface_present_modes(&vk.surface, Default::default())
                .map_err(Error::GetSurfaceCapabilites)?;

            let swapchain_image_count = surface_capabilities.min_image_count + 1;
            log::debug!("Swapchain images: {}", swapchain_image_count);

            let mailbox_score = if swapchain_image_count > 2 { 0 } else { 1 };
            let immediate_score = if swapchain_image_count <= 2 { 0 } else { 1 };

            // FIFO modes end up lagging behind to mailbox or immediate are preferred
            // As mailbox acts like FIFO with 2 or fewer images, in that case we should use immediate
            // Mailbox with > 2 images is preferred over immediate as it has less tearing
            let present_mode = present_modes
                .min_by_key(|mode| match mode {
                    vulkano::swapchain::PresentMode::Mailbox => mailbox_score,
                    vulkano::swapchain::PresentMode::Immediate => immediate_score,
                    vulkano::swapchain::PresentMode::FifoRelaxed => 2,
                    vulkano::swapchain::PresentMode::Fifo => 3,
                    _ => 4,
                })
                .expect("Device has no present modes");

            log::debug!("Present mode: {:?}", present_mode);

            Swapchain::new(
                vk.device.clone(),
                vk.surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: swapchain_image_count,
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    present_mode,
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

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let capture_pipeline = capture_pipeline::create_pipeline(vk, subpass.clone())?;
        let capture = CaptureObject::new(vk, capture_pipeline)?;

        let selection_pipeline = selection_pipeline::create_pipeline(vk, subpass.clone())?;
        let selection = Selection::new(vk, selection_pipeline)?;

        let border_pipeline = border_pipeline::create_pipeline(vk, subpass.clone())?;
        let selection_border = Border::new(vk, border_pipeline.clone(), [255, 255, 255, 255], 2.0)?;

        let mouse_pipeline = mouse_pipeline::create_pipeline(vk, subpass.clone())?;
        let mouse = Mouse::new(vk, mouse_pipeline, 1.0)?;

        let text_pipeline = parameters_pipeline::create_pipeline(vk, subpass.clone())?;
        let parameters = Parameters::new(vk, text_pipeline, 64)?;

        let rect_pipeline = rect_pipeline::create_pipeline(vk, subpass.clone())?;
        let text_rect = Rect::new(vk, rect_pipeline, [45, 55, 72, 255])?;

        let text_border = Border::new(vk, border_pipeline.clone(), [23, 25, 35, 255], 1.0)?;

        Ok(Self {
            framebuffers,
            viewport,
            swapchain,
            render_pass,
            capture,
            selection,
            selection_border,
            mouse,
            parameters,
            text_rect,
            text_border,
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

    #[error("Failed to create renderpass:\n{0:?}")]
    CreateRenderpass(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    CreateDescriptorSet(#[source] Validated<VulkanError>),

    #[error("Failed to perform Window Size Dependent Setup:\n{0}")]
    WindowSizeDependentSetup(#[from] window_size_dependent_setup::Error),

    #[error("Failed to create capture pipeline:\n{0}")]
    CapturePipeline(#[from] capture_pipeline::Error),

    #[error("Failed to create capture object:\n{0}")]
    CaptureObject(#[from] capture_pipeline::capture::Error),

    #[error("Failed to create selection shading pipeline:\n{0}")]
    SelectionShadingPipeline(#[from] selection_pipeline::Error),

    #[error("Failed to create selection shading object:\n{0}")]
    SelectionShadingObject(#[from] selection_pipeline::selection::Error),

    #[error("Failed to create border pipeline:\n{0}")]
    BorderPipeline(#[from] border_pipeline::Error),

    #[error("Failed to create border object:\n{0}")]
    BorderObject(#[from] border_pipeline::border::Error),

    #[error("Failed to create mouse pipeline:\n{0}")]
    MousePipeline(#[from] mouse_pipeline::Error),

    #[error("Failed to create mouse object:\n{0}")]
    MouseObject(#[from] mouse_pipeline::mouse::Error),

    #[error("Failed to create text pipeline:\n{0}")]
    TextPipeline(#[from] parameters_pipeline::Error),

    #[error("Failed to create text renderer:\n{0}")]
    TextRenderer(#[from] parameters_pipeline::parameters::Error),

    #[error("Failed to create rect pipeline:\n{0}")]
    RectPipeline(#[from] rect_pipeline::Error),

    #[error("Failed to create rect:\n{0}")]
    Rect(#[from] rect_pipeline::rect::Error),
}
