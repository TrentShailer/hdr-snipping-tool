// pub mod allocators;
// pub mod copy_buffer;
pub mod vulkan_instance;

use ash::{
    ext::debug_utils,
    khr::surface,
    vk::{DebugUtilsMessengerEXT, PhysicalDevice, Queue, SurfaceKHR},
    Device, Entry, Instance,
};

/// Bundled objects required to work with vulkan.
pub struct VulkanInstance {
    pub entry: Entry,
    pub instance: Instance,

    pub physical_device: PhysicalDevice,

    pub device: Device,

    pub queue: Queue,
    pub queue_family_index: u32,

    pub surface_loader: surface::Instance,
    pub surface: SurfaceKHR,

    pub debug_utils_loader: debug_utils::Instance,
    pub debug_messenger: DebugUtilsMessengerEXT,
}
