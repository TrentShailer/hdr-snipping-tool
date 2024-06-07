use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    swapchain::Surface,
    LoadingError, Validated, VulkanError, VulkanLibrary,
};
use winit::event_loop::ActiveEventLoop;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load vulkan library:\n{0}")]
    VulkanLibrary(#[from] LoadingError),

    #[error("Failed to create vulkan instance:\n{0:?}")]
    NewInstance(#[from] Validated<VulkanError>),
}

pub fn aquire_instance(event_loop: &ActiveEventLoop) -> Result<Arc<Instance>, Error> {
    let library = VulkanLibrary::new()?;

    let required_extensions = Surface::required_extensions(&event_loop);

    let instance_create_info = InstanceCreateInfo {
        flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
        enabled_extensions: required_extensions,
        ..Default::default()
    };
    Ok(Instance::new(library, instance_create_info)?)
}
