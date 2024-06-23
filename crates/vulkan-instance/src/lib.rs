pub mod allocators;
pub mod copy_buffer;
pub mod texture;
pub mod vulkan_instance;

use std::sync::Arc;

use allocators::Allocators;
use vulkano::{
    device::{Device, Queue},
    swapchain::Surface,
};

pub struct VulkanInstance {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface>,
    pub allocators: Arc<Allocators>,
}
