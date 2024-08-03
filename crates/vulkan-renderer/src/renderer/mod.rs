pub mod render;
pub mod units;
pub mod window_size_dependent_setup;

use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    image::{view::ImageView, ImageUsage},
    pipeline::graphics::{subpass::PipelineRenderingCreateInfo, viewport::Viewport},
    swapchain::{CompositeAlpha, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo},
    sync::{self, GpuFuture},
    Validated, VulkanError,
};
use window_size_dependent_setup::window_size_dependent_setup;
use winit::window::Window;

use crate::{capture::Capture, mouse_guides::MouseGuides, pipelines, selection::Selection};

pub struct Renderer {
    pub recreate_swapchain: bool,
    pub render_future: Option<Box<dyn GpuFuture>>,
    pub aquire_future: Option<SwapchainAcquireFuture>,
    pub swapchain: Arc<Swapchain>,
    pub attachment_views: Vec<Arc<ImageView>>,
    pub viewport: Viewport,
    //
    pub capture: Capture,
    pub selection: Selection,
    pub mouse_guides: MouseGuides,
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
                .min_by_key(|composite_alpha| match composite_alpha {
                    CompositeAlpha::Opaque => 0,
                    CompositeAlpha::PreMultiplied => 1,
                    CompositeAlpha::PostMultiplied => 2,
                    CompositeAlpha::Inherit => 3,
                    _ => 4,
                })
                .ok_or(Error::CompositeAlpha)?;

            let present_modes = vk
                .device
                .physical_device()
                .surface_present_modes(&vk.surface, Default::default())
                .map_err(Error::GetSurfaceCapabilites)?;

            let swapchain_image_count = surface_capabilities.min_image_count + 1;

            let present_mode = present_modes
                .min_by_key(|mode| match mode {
                    vulkano::swapchain::PresentMode::Fifo => 0,
                    vulkano::swapchain::PresentMode::Mailbox => 1,
                    vulkano::swapchain::PresentMode::FifoRelaxed => 2,
                    vulkano::swapchain::PresentMode::Immediate => 3,
                    _ => 4,
                })
                .expect("Device has no present modes");

            log::debug!(
                "[Renderer]
  Surface format: {:?}
  Swapchain images: {}
  Present mode: {:?}
  Composite alpha: {:?}",
                image_format,
                swapchain_image_count,
                present_mode,
                composite_alpha
            );

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

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let attachment_views = window_size_dependent_setup(&images, &mut viewport)?;

        let subpass = PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(swapchain.image_format())],
            ..Default::default()
        };

        // Create pipelines
        let capture_pipeline = pipelines::capture::create_pipeline(vk, subpass.clone())
            .map_err(|e| Error::Pipeline(e, "capture"))?;

        let selection_shading_pipeline =
            pipelines::selection_shading::create_pipeline(vk, subpass.clone())
                .map_err(|e| Error::Pipeline(e, "selection shading"))?;

        let border_pipeline = pipelines::border::create_pipeline(vk, subpass.clone())
            .map_err(|e| Error::Pipeline(e, "border"))?;

        let mouse_guides_pipeline = pipelines::mouse_guides::create_pipeline(vk, subpass.clone())
            .map_err(|e| Error::Pipeline(e, "mouse guide"))?;

        // Objects
        let capture =
            Capture::new(vk, capture_pipeline).map_err(|e| Error::Object(e, "capture"))?;

        let selection = Selection::new(vk, selection_shading_pipeline, border_pipeline.clone())
            .map_err(|e| Error::Object(e, "selection"))?;

        let mouse_guides = MouseGuides::new(vk, mouse_guides_pipeline, 1.0)
            .map_err(|e| Error::Object(e, "mouse guides"))?;

        Ok(Self {
            viewport,
            swapchain,
            attachment_views,
            aquire_future: None,
            recreate_swapchain: false,
            //
            capture,
            selection,
            mouse_guides,
            //
            render_future: Some(sync::now(vk.device.clone()).boxed()),
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

    //
    #[error("Failed to create {1} pipeline:\n{0}")]
    Pipeline(#[source] pipelines::Error, &'static str),

    #[error("Failed to create {1} render object:\n{0}")]
    Object(#[source] crate::vertex_index_buffer::Error, &'static str),
}
