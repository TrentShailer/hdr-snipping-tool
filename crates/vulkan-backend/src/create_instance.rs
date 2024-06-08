pub mod aquire_instance;
pub mod logical_device;
pub mod physical_device;
pub mod requirements;

use std::sync::Arc;

use logical_device::get_logical_device;
use physical_device::get_physical_device;
use thiserror::Error;
use vulkano::{swapchain::Surface, Validated, VulkanError};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    allocators::Allocators,
    renderer::{self},
    tonemapper::{self},
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
        let allocators = Arc::new(Allocators::new(device.clone()));

        let queue = queues.next().unwrap();

        Ok(Self {
            allocators,
            device,
            queue,
            surface,
        })
    }
}
