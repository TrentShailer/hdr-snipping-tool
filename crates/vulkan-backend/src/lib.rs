pub mod create_instance;
pub mod renderer;
pub mod texture;
pub mod tonemapper;

use std::sync::Arc;

use renderer::Renderer;
use tonemapper::Tonemapper;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, Queue},
    memory::allocator::StandardMemoryAllocator,
};

pub struct VulkanInstance {
    device: Arc<Device>,
    queue: Arc<Queue>,
    mem_alloc: Arc<StandardMemoryAllocator>,
    cb_alloc: Arc<StandardCommandBufferAllocator>,
    ds_alloc: Arc<StandardDescriptorSetAllocator>,
    pub tonemapper: Tonemapper,
    pub renderer: Renderer,
}
