pub mod buffer;
pub mod command_buffer;
pub mod create;
pub mod default;
pub mod memory;
pub mod shader;

use ash::{
    ext::debug_utils,
    khr::surface,
    vk::{self},
    Device, Entry, Instance,
};

pub use create::Error as CreateError;
use thiserror::Error;

/// Bundled objects required to work with vulkan.
pub struct VulkanInstance {
    #[allow(unused)]
    entry: Entry,
    pub instance: Instance,

    pub physical_device: vk::PhysicalDevice,
    pub device: Device,

    pub queue: vk::Queue,
    pub queue_family_index: u32,

    pub surface_loader: surface::Instance,
    pub surface: vk::SurfaceKHR,

    pub command_buffer_pool: vk::CommandPool,
    pub command_buffer: (vk::CommandBuffer, vk::Fence),

    debug_utils: Option<(debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VulkanError {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    VkResult(#[source] vk::Result, &'static str),

    #[error("No suitable memory type found")]
    NoSuitableMemoryType,
}
