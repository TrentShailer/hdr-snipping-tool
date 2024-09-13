mod drop;
pub mod import;

use std::sync::Arc;

use ash::vk::{DeviceMemory, Image, ImageView};
use thiserror::Error;
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::maximum;

/// An Hdr Caputre Vulkan Object
#[non_exhaustive]
pub struct HdrCapture {
    vk: Arc<VulkanInstance>,

    image: Image,
    memory: DeviceMemory,

    /// The extent of the capture
    pub size: [u32; 2],

    /// The capture's whitepoint
    pub whitepoint: f32,

    /// The view of the capture
    pub image_view: ImageView,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Vulkan(#[from] VulkanError),

    #[error("Failed to find maximum value in capture:\n{0}")]
    Maximum(#[from] maximum::Error),
}
