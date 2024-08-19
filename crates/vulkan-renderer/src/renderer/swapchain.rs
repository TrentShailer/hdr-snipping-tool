use ash::vk::{
    AccessFlags2, ColorSpaceKHR, CompositeAlphaFlagsKHR, DependencyInfoKHR, Extent2D, Format,
    Image, ImageAspectFlags, ImageLayout, ImageMemoryBarrier2, ImageSubresourceRange,
    ImageUsageFlags, ImageView, ImageViewCreateInfo, ImageViewType, PipelineStageFlags2,
    PresentModeKHR, SharingMode, SurfaceFormatKHR, SurfaceTransformFlagsKHR,
    SwapchainCreateInfoKHR, SwapchainKHR, Viewport,
};
use thiserror::Error;
use vulkan_instance::{record_submit_command_buffer, VulkanInstance};

use super::Renderer;

impl Renderer {
    pub fn get_surface_format(vk: &VulkanInstance) -> Result<SurfaceFormatKHR, ash::vk::Result> {
        let surface_formats = unsafe {
            vk.surface_loader
                .get_physical_device_surface_formats(vk.physical_device, vk.surface)?
        };

        let surface_format = surface_formats
                .into_iter()
                .min_by_key(|sufrace_format| match sufrace_format.format {
                    Format::R16G16B16A16_SFLOAT => 0,
                    _ => 1,
                } +
                match sufrace_format.color_space {
                    ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT => 0,
                    _ => 1,
                }
            ).unwrap();

        Ok(surface_format)
    }

    pub fn create_swapchain(
        vk: &VulkanInstance,
        swapchain_loader: &ash::khr::swapchain::Device,
        window_size: [u32; 2],
        old_swapchain: Option<SwapchainKHR>,
    ) -> Result<SwapchainKHR, Error> {
        let surface_format =
            Self::get_surface_format(vk).map_err(|e| Error::Vulkan(e, "getting surface format"))?;

        let surface_capabilities = unsafe {
            vk.surface_loader
                .get_physical_device_surface_capabilities(vk.physical_device, vk.surface)
                .map_err(|e| Error::Vulkan(e, "querying surface capabilities"))?
        };

        let mut swapchain_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && swapchain_image_count > surface_capabilities.max_image_count
        {
            swapchain_image_count = surface_capabilities.max_image_count;
        }

        let present_mode = {
            let present_modes = unsafe {
                vk.surface_loader
                    .get_physical_device_surface_present_modes(vk.physical_device, vk.surface)
                    .map_err(|e| Error::Vulkan(e, "querying surface present modes"))?
            };

            present_modes
                .into_iter()
                .min_by_key(|&mode| match mode {
                    PresentModeKHR::FIFO => 0,
                    PresentModeKHR::MAILBOX => 1,
                    PresentModeKHR::FIFO_RELAXED => 2,
                    PresentModeKHR::IMMEDIATE => 3,
                    _ => 4,
                })
                .expect("Device has no present modes")
        };

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(SurfaceTransformFlagsKHR::IDENTITY)
        {
            SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let extent = Extent2D {
            width: window_size[0],
            height: window_size[1],
        };

        let swapchain_create_info = SwapchainCreateInfoKHR::default()
            .surface(vk.surface)
            .min_image_count(swapchain_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(extent)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain_create_info = if let Some(old_swapchain) = old_swapchain {
            swapchain_create_info.old_swapchain(old_swapchain)
        } else {
            swapchain_create_info
        };

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }
            .map_err(|e| Error::Vulkan(e, "creating the swapchain"))?;

        Ok(swapchain)
    }

    pub fn transition_images(vk: &VulkanInstance, images: &[Image]) -> Result<(), Error> {
        vk.record_submit_command_buffer(
            vulkan_instance::CommandBufferUsage::Draw,
            &[],
            &[],
            |device, command_buffer| {
                unsafe {
                    let barriers: Box<[ImageMemoryBarrier2]> = images
                        .iter()
                        .map(|&image| {
                            ImageMemoryBarrier2::default()
                                .image(image)
                                .dst_access_mask(
                                    AccessFlags2::COLOR_ATTACHMENT_READ
                                        | AccessFlags2::COLOR_ATTACHMENT_WRITE,
                                )
                                .dst_stage_mask(PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                                .old_layout(ImageLayout::UNDEFINED)
                                .new_layout(ImageLayout::PRESENT_SRC_KHR)
                                .subresource_range(
                                    ImageSubresourceRange::default()
                                        .aspect_mask(ImageAspectFlags::COLOR)
                                        .layer_count(1)
                                        .level_count(1),
                                )
                        })
                        .collect();
                    let dependency_info =
                        DependencyInfoKHR::default().image_memory_barriers(&barriers);
                    device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
                }
                Ok(())
            },
        )?;

        let fences = [*vk
            .fences
            .get(&vulkan_instance::CommandBufferUsage::Draw)
            .unwrap()];
        unsafe {
            vk.device
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(|e| Error::Vulkan(e, "waiting for fences"))?;
        }

        Ok(())
    }

    pub fn recreate_swapchain(
        &mut self,
        vk: &VulkanInstance,
        window_size: [u32; 2],
    ) -> Result<(), Error> {
        unsafe {
            vk.device
                .device_wait_idle()
                .map_err(|e| Error::Vulkan(e, "waiting for device idle"))?;
        }

        let new_swapchain = Self::create_swapchain(
            vk,
            &self.swapchain_loader,
            window_size,
            Some(self.swapchain),
        )?;
        self.cleanup_swapchain();
        self.swapchain = new_swapchain;

        let swapchain_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(self.swapchain)
                .map_err(|e| Error::Vulkan(e, "getting swapchain images"))?
        };
        self.attachment_images = swapchain_images;
        Self::transition_images(vk, &self.attachment_images)?;

        let attachment_views = Self::window_size_dependant_setup(
            vk,
            &self.attachment_images,
            window_size,
            &mut self.viewport,
        )?;
        self.attachment_views = attachment_views;

        // TODO self.aquire_future = None;

        self.recreate_swapchain = false;
        Ok(())
    }

    pub fn cleanup_swapchain(&mut self) {
        unsafe {
            self.attachment_views
                .iter()
                .for_each(|&view| self.device.destroy_image_view(view, None));
            self.attachment_images.clear();

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn window_size_dependant_setup(
        vk: &VulkanInstance,
        images: &[Image],
        window_size: [u32; 2],
        viewport: &mut Viewport,
    ) -> Result<Vec<ImageView>, Error> {
        let image_format =
            Self::get_surface_format(vk).map_err(|e| Error::Vulkan(e, "getting surface format"))?;

        viewport.width = window_size[0] as f32;
        viewport.height = window_size[1] as f32;

        let image_views: Result<Vec<ImageView>, Error> = images
            .iter()
            .map(|&image| {
                let create_view_info = ImageViewCreateInfo::default()
                    .view_type(ImageViewType::TYPE_2D)
                    .format(image_format.format)
                    .subresource_range(ImageSubresourceRange {
                        aspect_mask: ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                unsafe {
                    vk.device
                        .create_image_view(&create_view_info, None)
                        .map_err(|e| Error::Vulkan(e, "creating image view"))
                }
            })
            .collect();

        image_views
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] record_submit_command_buffer::Error),
}
