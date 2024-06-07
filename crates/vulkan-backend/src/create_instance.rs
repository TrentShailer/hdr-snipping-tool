pub mod aquire_instance;
pub mod logical_device;
pub mod physical_device;
pub mod requirements;

use std::sync::Arc;

use logical_device::get_logical_device;
use physical_device::get_physical_device;
use thiserror::Error;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    memory::allocator::StandardMemoryAllocator, swapchain::Surface, Validated, VulkanError,
};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    renderer::{self, Renderer},
    tonemapper::{self, Tonemapper},
    VulkanInstance,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to aquire vulkan instance:\n{0}")]
    AquireInstance(#[from] aquire_instance::Error),

    #[error("Failed to create surface:\n{0:?}")]
    NewSurface(#[source] Validated<VulkanError>),

    #[error("Failed to find suitable physical device:\n{0}")]
    PhysicalDevice(#[from] physical_device::Error),

    #[error("Failed to create logical device:\n{0:?}")]
    LogicalDevice(#[source] Validated<VulkanError>),

    #[error("Failed to create tonemapper:\n{0}")]
    Tonemapper(#[from] tonemapper::Error),

    #[error("Failed to create renderer:\n{0}")]
    Renderer(#[from] renderer::Error),
}

impl VulkanInstance {
    pub fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Result<Self, Error> {
        let instance = aquire_instance::aquire_instance(event_loop)?;
        log::info!("Vulkan {}", instance.api_version());

        let surface =
            Surface::from_window(instance.clone(), window.clone()).map_err(Error::NewSurface)?;

        let (physical_device, queue_family_index) =
            get_physical_device(instance.clone(), surface.clone())?;
        log::info!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = get_logical_device(physical_device, queue_family_index)
            .map_err(Error::LogicalDevice)?;

        // Create memory allocators
        let mem_alloc = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let ds_alloc = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));
        let cb_alloc = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let queue = queues.next().unwrap(); // Unwrap is safe as long as queue count is correct

        let tonemapper = Tonemapper::new(
            device.clone(),
            queue.clone(),
            mem_alloc.clone(),
            ds_alloc.clone(),
            cb_alloc.clone(),
        )?;

        let renderer = Renderer::new(
            device.clone(),
            queue.clone(),
            mem_alloc.clone(),
            cb_alloc.clone(),
            surface.clone(),
            window.clone(),
        )?;

        Ok(Self {
            device,
            queue,
            mem_alloc,
            cb_alloc,
            ds_alloc,
            tonemapper,
            renderer,
        })
    }
}
