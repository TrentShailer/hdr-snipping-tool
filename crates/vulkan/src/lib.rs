//! # Vulkan
//! Contains all the Vulkan components for HDR Snipping Tool.
//!

#![warn(missing_docs)]

extern crate alloc;

pub use hdr_image::HdrImage;
pub use hdr_scanner::{HdrScanner, HdrScannerError};
pub use hdr_to_sdr_tonemapper::{HdrToSdrTonemapper, TonemapperError};
pub use renderer::{CreationError as RendererCreationError, Renderer, State as RendererState};
pub use sdr_image::{SdrImage, SdrImageError};
pub use vulkan::{QueuePurpose, Vulkan, VulkanCreationError};

mod hdr_image;
mod hdr_scanner;
mod hdr_to_sdr_tonemapper;
mod renderer;
mod sdr_image;
mod vulkan;
