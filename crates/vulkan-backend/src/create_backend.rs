use std::sync::Arc;

use thiserror::Error;
use winit::window::Window;

use crate::{
    renderer::{self, Renderer},
    tonemapper::{self, Tonemapper},
    VulkanBackend, VulkanInstance,
};

impl VulkanBackend {
    pub fn new(instance: &VulkanInstance, window: Arc<Window>) -> Result<Self, Error> {
        let tonemapper = Tonemapper::new(&instance)?;

        let renderer = Renderer::new(&instance, window.clone())?;

        Ok(Self {
            tonemapper,
            renderer,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create tonemapper:\n{0}")]
    Tonemapper(#[from] tonemapper::Error),

    #[error("Failed to create renderer:\n{0}")]
    Renderer(#[from] renderer::Error),
}
