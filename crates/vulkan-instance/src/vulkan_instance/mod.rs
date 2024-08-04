mod aquire_instance;
mod logical_device;
mod physical_device;
mod requirements;

use std::sync::Arc;

use logical_device::get_logical_device;
use physical_device::get_physical_device;
use thiserror::Error;
use tracing::{info, info_span};
use vulkano::{swapchain::Surface, Validated, VulkanError};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{allocators::Allocators, VulkanInstance};

impl VulkanInstance {
    pub fn new(window: Arc<Window>, event_loop: &ActiveEventLoop) -> Result<Self, Error> {
        let _span = info_span!("VulkanInstance::new").entered();

        let instance = aquire_instance::aquire_instance(event_loop)?;
        info!("API v{:?}", instance.api_version());

        let surface =
            Surface::from_window(instance.clone(), window.clone()).map_err(Error::NewSurface)?;

        let (physical_device, queue_family_index, feature_extensions) =
            get_physical_device(instance.clone(), surface.clone())?;

        {
            info!(
                "{} ({:?})",
                physical_device.properties().device_name,
                physical_device.properties().device_type
            );

            info!("Queue family: {}", queue_family_index);
            info!("Feature extensions: {:?}", feature_extensions);
        }

        let (device, mut queues) = get_logical_device(
            physical_device.clone(),
            queue_family_index,
            feature_extensions,
        )
        .map_err(Error::LogicalDevice)?;

        let allocators = Arc::new(Allocators::new(device.clone()));

        let queue = queues.next().unwrap();

        let vk = Self {
            allocators,
            physical_device,
            device,
            queue,
            surface,
        };

        Ok(vk)
    }
}

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
}
