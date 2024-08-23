use std::sync::Arc;

use ash::vk::{
    Fence, FenceCreateInfo, PipelineRenderingCreateInfo, Semaphore, SemaphoreCreateInfo, Viewport,
};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::{
    objects::{Capture, MouseGuides, Selection},
    pipelines, Error, Renderer,
};

impl Renderer {
    #[instrument("Renderer::new", skip_all, err)]
    pub fn new(vk: Arc<VulkanInstance>, window_size: [u32; 2]) -> Result<Self, Error> {
        let swapchain_loader = ash::khr::swapchain::Device::new(&vk.instance, &vk.device);
        let swapchain = Self::create_swapchain(&vk, &swapchain_loader, window_size, None)?;
        let swapchain_images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(|e| VulkanError::VkResult(e, "getting swapchain images"))?
        };
        Self::transition_images(&vk, &swapchain_images)?;

        let mut viewport = Viewport::default().min_depth(0.0).max_depth(1.0);
        let attachment_views =
            Self::window_size_dependant_setup(&vk, &swapchain_images, window_size, &mut viewport)?;

        let surface_format = Self::get_surface_format(&vk)
            .map_err(|e| VulkanError::VkResult(e, "getting surface format"))?;

        let surface_formats = [surface_format.format];
        let pipeline_rendering_create_info =
            PipelineRenderingCreateInfo::default().color_attachment_formats(&surface_formats);

        // Create pipelines
        let capture_pipeline =
            pipelines::capture::create_pipeline(&vk, pipeline_rendering_create_info, viewport)?;

        let selection_shading_pipeline = pipelines::selection_shading::create_pipeline(
            &vk,
            pipeline_rendering_create_info,
            viewport,
        )?;

        let border_pipeline =
            pipelines::border::create_pipeline(&vk, pipeline_rendering_create_info, viewport)?;

        let mouse_guides_pipeline = pipelines::mouse_guides::create_pipeline(
            &vk,
            pipeline_rendering_create_info,
            viewport,
        )?;

        // Objects
        let capture = Capture::new(
            vk.clone(),
            capture_pipeline.0,
            capture_pipeline.1,
            capture_pipeline.3,
        )?;

        let selection = Selection::new(
            vk.clone(),
            selection_shading_pipeline.0,
            selection_shading_pipeline.1,
            border_pipeline.0,
            border_pipeline.1,
        )?;

        let mouse_guides = MouseGuides::new(
            vk.clone(),
            mouse_guides_pipeline.0,
            mouse_guides_pipeline.1,
            1.0,
        )?;

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

        Ok(Self {
            vk,

            command_buffers,
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

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.descriptor_layouts
                .iter()
                .for_each(|&layout| self.vk.device.destroy_descriptor_set_layout(layout, None));
            self.shaders
                .iter()
                .for_each(|&shader| self.vk.device.destroy_shader_module(shader, None));
            self.pipeline_layouts
                .iter()
                .for_each(|&layout| self.vk.device.destroy_pipeline_layout(layout, None));
            self.pipelines
                .iter()
                .for_each(|&pipeline| self.vk.device.destroy_pipeline(pipeline, None));

            self.cleanup_swapchain();
        }
    }
}
