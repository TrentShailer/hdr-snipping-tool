pub mod buffer;
pub mod find_memory_type_index;
pub mod record_submit_command_buffer;
pub mod vulkan_instance;

use std::sync::Arc;

use ash::{
    ext::debug_utils,
    khr::surface,
    vk::{
        CommandBuffer, CommandPool, DebugUtilsMessengerEXT, Fence, PhysicalDevice, Queue,
        SurfaceKHR,
    },
    Device, Entry, Instance,
};

/// Bundled objects required to work with vulkan.
pub struct VulkanInstance {
    pub entry: Entry,
    pub instance: Instance,

    pub physical_device: PhysicalDevice,
    pub device: Arc<Device>,

    pub queue: Queue,
    pub queue_family_index: u32,

    pub surface_loader: surface::Instance,
    pub surface: SurfaceKHR,

    pub command_buffer_pool: CommandPool,
    pub command_buffer: CommandBuffer,
    pub fence: Fence,

    pub debug_utils_loader: debug_utils::Instance,
    pub debug_messenger: DebugUtilsMessengerEXT,
}
