// pub mod allocators;
// pub mod copy_buffer;
pub mod find_memory_type_index;
pub mod record_submit_command_buffer;
pub mod vulkan_instance;

use std::{collections::HashMap, sync::Arc};

use ash::{
    ext::debug_utils,
    khr::surface,
    vk::{
        CommandBuffer, CommandPool, DebugUtilsMessengerEXT, Fence, PhysicalDevice, Queue,
        Semaphore, SurfaceKHR,
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
    pub command_buffers: HashMap<CommandBufferUsage, CommandBuffer>,
    pub fences: HashMap<CommandBufferUsage, Fence>,
    pub semaphores: HashMap<SemaphoreUsage, Semaphore>,

    pub debug_utils_loader: debug_utils::Instance,
    pub debug_messenger: DebugUtilsMessengerEXT,
}

#[derive(PartialEq, Eq, Hash)]
pub enum CommandBufferUsage {
    Draw,
    Setup,
    Tonemap,
}
impl CommandBufferUsage {
    pub const VALUES: [CommandBufferUsage; 3] = [
        CommandBufferUsage::Draw,
        CommandBufferUsage::Setup,
        CommandBufferUsage::Tonemap,
    ];
}

#[derive(PartialEq, Eq, Hash)]
pub enum SemaphoreUsage {
    Render,
    Present,
}
impl SemaphoreUsage {
    pub const VALUES: [SemaphoreUsage; 2] = [SemaphoreUsage::Render, SemaphoreUsage::Present];
}
