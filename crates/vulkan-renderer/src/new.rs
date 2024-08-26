use std::sync::Arc;

use ash::vk::{
    ColorSpaceKHR, Fence, FenceCreateInfo, PipelineRenderingCreateInfo, Semaphore,
    SemaphoreCreateInfo, Viewport,
};
use tracing::{error, info_span, instrument};
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::{
    objects::{Capture, MouseGuides, Selection},
    pipelines::{
        border::BorderPipeline, capture::CapturePipeline, mouse_guides::MouseGuidesPipeline,
        selection_shading::SelectionShadingPipeline,
    },
    Error, Renderer,
};

impl Renderer {
    #[instrument("Renderer::new", skip_all, err)]
    pub fn new(vk: Arc<VulkanInstance>, window_size: [u32; 2]) -> Result<Self, Error> {
        let swapchain_loader = ash::khr::swapchain::Device::new(&vk.instance, &vk.device);
        let (swapchain, swapchain_format) =
            Self::create_swapchain(&vk, &swapchain_loader, window_size, None)?;
        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(|e| VulkanError::VkResult(e, "getting swapchain images"))?
        };
        Self::transition_images(&vk, &swapchain_images)?;

        let mut viewport = Viewport::default().min_depth(0.0).max_depth(1.0);
        let attachment_views =
            Self::window_size_dependant_setup(&vk, &swapchain_images, window_size, &mut viewport)?;

        let surface_formats = [swapchain_format.format];
        let pipeline_rendering_create_info =
            PipelineRenderingCreateInfo::default().color_attachment_formats(&surface_formats);

        // Create pipelines
        let capture_pipeline =
            CapturePipeline::new(vk.clone(), pipeline_rendering_create_info, viewport)?;
        let selection_shading_pipeline =
            SelectionShadingPipeline::new(vk.clone(), pipeline_rendering_create_info, viewport)?;
        let border_pipeline =
            BorderPipeline::new(vk.clone(), pipeline_rendering_create_info, viewport)?;
        let mouse_guides_pipeline =
            MouseGuidesPipeline::new(vk.clone(), pipeline_rendering_create_info, viewport)?;

        // Objects
        let capture = Capture::new(vk.clone(), capture_pipeline.descriptor_layouts)?;
        let selection = Selection::new(vk.clone())?;
        let mouse_guides = MouseGuides::new(vk.clone(), 1.0)?;

        let sync_item_count = swapchain_images.len() as u32;
        // create command buffers
        let command_buffers = vk.allocate_command_buffers(sync_item_count)?;

        // create acquire fences
        let fence_create_info = FenceCreateInfo::default();
        let acquire_fences: Vec<Fence> = (0..sync_item_count)
            .map(|_| unsafe {
                vk.device
                    .create_fence(&fence_create_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating fence"))
            })
            .collect::<Result<_, VulkanError>>()?;

        // create render semaphores
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let render_semaphores: Vec<Semaphore> = (0..sync_item_count)
            .map(|_| unsafe {
                vk.device
                    .create_semaphore(&semaphore_create_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating semaphore"))
            })
            .collect::<Result<_, VulkanError>>()?;

        let non_linear_swapchain = swapchain_format.color_space == ColorSpaceKHR::SRGB_NONLINEAR;

        Ok(Self {
            vk,

            command_buffers,
            acquire_fences,
            render_semaphores,

            swapchain_loader,
            swapchain,
            non_linear_swapchain,

            attachment_images: swapchain_images,
            attachment_views,

            viewport,

            capture,
            selection,
            mouse_guides,

            capture_pipeline,
            selection_shading_pipeline,
            border_pipeline,
            mouse_guides_pipeline,

            recreate_swapchain: false,
        })
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _span = info_span!("Renderer::Drop").entered();
        unsafe {
            if self.vk.device.device_wait_idle().is_err() {
                error!("Failed to wait for device idle on drop");
                return;
            }

            self.command_buffers
                .iter()
                .for_each(|(_, fence)| self.vk.device.destroy_fence(*fence, None));
            self.acquire_fences
                .iter()
                .for_each(|fence| self.vk.device.destroy_fence(*fence, None));
            self.render_semaphores
                .iter()
                .for_each(|semaphore| self.vk.device.destroy_semaphore(*semaphore, None));

            self.cleanup_swapchain();
        }
    }
}
