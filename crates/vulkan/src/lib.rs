//! # Vulkan
//! Contains all the Vulkan components for HDR Snipping Tool.
//!

#![warn(missing_docs)]

extern crate alloc;

pub use hdr_image::HdrImage;
pub use hdr_scanner::{Error as HdrScannerError, HdrScanner};
pub use hdr_to_sdr_tonemapper::{Error as TonemapperError, HdrToSdrTonemapper};
pub use renderer::{CreationError as RendererCreationError, Renderer, State as RendererState};
pub use sdr_image::SdrImage;
pub use vulkan::{Error as VulkanCreationError, QueuePurpose, Vulkan};

mod hdr_image;
mod hdr_scanner;
mod hdr_to_sdr_tonemapper;
mod renderer;
mod sdr_image;
mod vulkan;
