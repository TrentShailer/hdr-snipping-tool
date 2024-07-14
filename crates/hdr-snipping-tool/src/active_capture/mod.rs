pub mod save;
pub mod selection;

use std::sync::Arc;

use scrgb::ScRGB;
use scrgb_tonemapper::ScrgbTonemapper;
use selection::Selection;
use thiserror::Error;
use vulkan_instance::{texture::Texture, VulkanInstance};
use windows_capture_provider::{capture::Capture, WindowsCaptureProvider};
use winit::dpi::PhysicalPosition;

pub struct ActiveCapture {
    pub capture: Capture,
    pub texture: Arc<Texture>,
    pub tonemapper: ScrgbTonemapper,
    pub selection: Selection,
}

impl ActiveCapture {
    pub fn new(
        vk: &VulkanInstance,
        capture_provider: &mut WindowsCaptureProvider,
    ) -> Result<Self, Error> {
        let capture = capture_provider.take_capture()?;
        let texture = Arc::new(Texture::new(vk, capture.display.size)?);
        let tonemapper = ScrgbTonemapper::new(vk, texture.image_view.clone(), &capture)?;
        tonemapper.tonemap(vk)?;

        let selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(capture.display.size[0], capture.display.size[1]),
        );

        Ok(Self {
            capture,
            selection,
            texture,
            tonemapper,
        })
    }

    /// Adjusts the whitepoint of the tonemapper by an amount.
    pub fn adjust_whitepoint(
        &mut self,
        vk: &VulkanInstance,
        amount: ScRGB,
    ) -> Result<ScRGB, scrgb_tonemapper::tonemap::Error> {
        self.tonemapper.whitepoint += amount;
        self.tonemapper.tonemap(vk)?;
        Ok(self.tonemapper.whitepoint)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to take capture:\n{0}")]
    TakeCapture(#[from] windows_capture_provider::capture::Error),

    #[error("Failed to create texture:\n{0}")]
    Texture(#[from] vulkan_instance::texture::Error),

    #[error("Failed to create tonemapper:\n{0}")]
    Tonemapper(#[from] scrgb_tonemapper::Error),

    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] scrgb_tonemapper::tonemap::Error),
}
