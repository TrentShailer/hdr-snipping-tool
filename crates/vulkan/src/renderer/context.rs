use ash::{khr, vk};
use ash_helper::{try_name, LabelledVkResult, SurfaceContext, VkError, VulkanContext};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::Vulkan;

pub struct Surface {
    surface: vk::SurfaceKHR,

    surface_instance: khr::surface::Instance,
    swapchain_device: khr::swapchain::Device,

    rendering_device: khr::dynamic_rendering::Device,
}

impl Surface {
    pub unsafe fn new(
        vulkan: &Vulkan,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> LabelledVkResult<Self> {
        let surface = unsafe {
            ash_window::create_surface(
                vulkan.entry(),
                vulkan.instance(),
                display_handle,
                window_handle,
                None,
            )
            .map_err(|e| VkError::new(e, "createSurface"))?
        };

        let surface_instance = khr::surface::Instance::new(vulkan.entry(), vulkan.instance());
        let swapchain_device = khr::swapchain::Device::new(vulkan.instance(), vulkan.device());
        let rendering_device =
            khr::dynamic_rendering::Device::new(vulkan.instance(), vulkan.device());

        // Name the objects
        unsafe {
            try_name(vulkan, surface_instance.instance(), "Surface Instance");
            try_name(vulkan, swapchain_device.device(), "Swapchain Device");
            try_name(
                vulkan,
                rendering_device.device(),
                "Dynamic Rendering Device",
            );
            try_name(vulkan, surface, "Surface");
        }

        Ok(Self {
            surface,
            surface_instance,
            swapchain_device,
            rendering_device,
        })
    }

    #[inline]
    pub unsafe fn rendering_device(&self) -> &khr::dynamic_rendering::Device {
        &self.rendering_device
    }
}

impl SurfaceContext for Surface {
    #[inline]
    unsafe fn surface_instance(&self) -> &khr::surface::Instance {
        &self.surface_instance
    }

    #[inline]
    unsafe fn swapchain_device(&self) -> &khr::swapchain::Device {
        &self.swapchain_device
    }

    #[inline]
    unsafe fn surface(&self) -> vk::SurfaceKHR {
        self.surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_instance.destroy_surface(self.surface, None);
        }
    }
}
