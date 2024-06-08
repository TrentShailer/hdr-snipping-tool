pub mod allocators;
pub mod create_backend;
pub mod create_instance;
pub mod renderer;
pub mod texture;
pub mod tonemapper;

use std::sync::Arc;

use allocators::Allocators;
use renderer::Renderer;
use tonemapper::Tonemapper;
use vulkano::{
    device::{Device, Queue},
    swapchain::Surface,
};

pub struct VulkanBackend {
    pub tonemapper: Tonemapper,
    pub renderer: Renderer,
}

pub struct VulkanInstance {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface>,
    pub allocators: Arc<Allocators>,
}
