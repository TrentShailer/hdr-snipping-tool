pub mod drop;
pub mod render;
pub mod swapchain;
pub mod units;

use std::sync::Arc;

use ash::{
    vk::{
        CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, DescriptorSetLayout, Fence,
        FenceCreateFlags, FenceCreateInfo, Image, ImageView, Pipeline, PipelineLayout,
        PipelineRenderingCreateInfo, Semaphore, SemaphoreCreateInfo, ShaderModule, SwapchainKHR,
        Viewport,
    },
    Device,
};
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::{record_submit_command_buffer, VulkanInstance};
use winit::window::Window;

use crate::{
    capture::{self, Capture},
    mouse_guides::MouseGuides,
    pipelines,
    selection::Selection,
};

pub struct Renderer {
    pub recreate_swapchain: bool,

    pub device: Arc<Device>,

    pub command_buffers: Vec<CommandBuffer>,
    pub cb_fences: Vec<Fence>,
    pub acquire_fences: Vec<Fence>,
    pub render_semaphores: Vec<Semaphore>,

    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: SwapchainKHR,

    pub attachment_images: Vec<Image>,
    pub attachment_views: Vec<ImageView>,

    pub viewport: Viewport,

    pub capture: Capture,
    pub selection: Selection,
    pub mouse_guides: MouseGuides,

    pub pipeline_layouts: Vec<PipelineLayout>,
    pub pipelines: Vec<Pipeline>,
    pub shaders: Vec<ShaderModule>,
    pub descriptor_layouts: Vec<DescriptorSetLayout>,
}

impl Renderer {
    pub fn new(vk: &VulkanInstance, window: Arc<Window>) -> Result<Self, Error> {
        let _span = info_span!("Renderer::new").entered();

        let window_size: [u32; 2] = window.inner_size().into();

        let swapchain_loader = ash::khr::swapchain::Device::new(&vk.instance, &vk.device);
        let swapchain = Self::create_swapchain(vk, &swapchain_loader, window_size, None)?;
        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(|e| Error::Vulkan(e, "getting swapchain images"))?
        };
        Self::transition_images(vk, &swapchain_images)?;

        let mut viewport = Viewport::default().min_depth(0.0).max_depth(1.0);
        let attachment_views =
            Self::window_size_dependant_setup(vk, &swapchain_images, window_size, &mut viewport)?;

        let surface_format =
            Self::get_surface_format(vk).map_err(|e| Error::Vulkan(e, "getting surface format"))?;
        let surface_formats = [surface_format.format];
        let pipeline_rendering_create_info =
            PipelineRenderingCreateInfo::default().color_attachment_formats(&surface_formats);

        // Create pipelines
        let capture_pipeline =
            pipelines::capture::create_pipeline(vk, pipeline_rendering_create_info, viewport)
                .map_err(|e| Error::Pipeline(e, "capture"))?;

        let selection_shading_pipeline = pipelines::selection_shading::create_pipeline(
            vk,
            pipeline_rendering_create_info,
            viewport,
        )
        .map_err(|e| Error::Pipeline(e, "selection shading"))?;

        let border_pipeline =
            pipelines::border::create_pipeline(vk, pipeline_rendering_create_info, viewport)
                .map_err(|e| Error::Pipeline(e, "border"))?;

        let mouse_guides_pipeline =
            pipelines::mouse_guides::create_pipeline(vk, pipeline_rendering_create_info, viewport)
                .map_err(|e| Error::Pipeline(e, "mouse guide"))?;

        // Objects
        let capture = Capture::new(
            vk,
            capture_pipeline.0,
            capture_pipeline.1,
            capture_pipeline.3,
        )?;

        let selection = Selection::new(
            vk,
            selection_shading_pipeline.0,
            selection_shading_pipeline.1,
            border_pipeline.0,
            border_pipeline.1,
        )
        .map_err(|e| Error::Object(e, "selection"))?;

        let mouse_guides =
            MouseGuides::new(vk, mouse_guides_pipeline.0, mouse_guides_pipeline.1, 1.0)
                .map_err(|e| Error::Object(e, "mouse guides"))?;

        let pipelines = vec![
            capture_pipeline.0,
            border_pipeline.0,
            mouse_guides_pipeline.0,
            selection_shading_pipeline.0,
        ];

        let pipeline_layouts = vec![
            capture_pipeline.1,
            border_pipeline.1,
            mouse_guides_pipeline.1,
            selection_shading_pipeline.1,
        ];

        let shaders = vec![
            capture_pipeline.2[0],
            capture_pipeline.2[1],
            border_pipeline.2[0],
            border_pipeline.2[1],
            mouse_guides_pipeline.2[0],
            mouse_guides_pipeline.2[1],
            selection_shading_pipeline.2[0],
            selection_shading_pipeline.2[1],
        ];

        let descriptor_layouts = vec![capture_pipeline.3[0], capture_pipeline.3[1]];

        let sync_item_count = swapchain_images.len() as u32;
        // create command buffers
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .command_buffer_count(sync_item_count)
            .command_pool(vk.command_buffer_pool)
            .level(CommandBufferLevel::PRIMARY);
        let command_buffers = unsafe {
            vk.device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| Error::Vulkan(e, "allocating command buffers"))?
        };

        // create command buffer fences
        let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);
        let cb_fences: Vec<Fence> = (0..sync_item_count)
            .map(|_| unsafe {
                vk.device
                    .create_fence(&fence_create_info, None)
                    .map_err(|e| Error::Vulkan(e, "creating fence"))
            })
            .collect::<Result<_, Error>>()?;

        // create acquire fences
        let fence_create_info = FenceCreateInfo::default();
        let acquire_fences: Vec<Fence> = (0..sync_item_count)
            .map(|_| unsafe {
                vk.device
                    .create_fence(&fence_create_info, None)
                    .map_err(|e| Error::Vulkan(e, "creating fence"))
            })
            .collect::<Result<_, Error>>()?;

        // create render semaphores
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let render_semaphores: Vec<Semaphore> = (0..sync_item_count)
            .map(|_| unsafe {
                vk.device
                    .create_semaphore(&semaphore_create_info, None)
                    .map_err(|e| Error::Vulkan(e, "creating semaphore"))
            })
            .collect::<Result<_, Error>>()?;

        Ok(Self {
            device: vk.device.clone(),

            command_buffers,
            cb_fences,
            acquire_fences,
            render_semaphores,

            swapchain_loader,
            swapchain,

            attachment_images: swapchain_images,
            attachment_views,

            viewport,

            capture,
            selection,
            mouse_guides,

            pipeline_layouts,
            pipelines,
            descriptor_layouts,
            shaders,

            recreate_swapchain: false,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] record_submit_command_buffer::Error),

    #[error("Failed to create swapchain:\n{0}")]
    Swapchain(#[from] swapchain::Error),

    //
    #[error("Failed to create {1} pipeline:\n{0}")]
    Pipeline(#[source] pipelines::Error, &'static str),

    #[error("Failed to create {1} render object:\n{0}")]
    Object(#[source] crate::vertex_index_buffer::Error, &'static str),

    #[error("Failed to create capture render object:\n{0}")]
    CaptureObject(#[from] capture::Error),
}
