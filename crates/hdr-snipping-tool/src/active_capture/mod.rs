pub mod save;
pub mod selection;

use std::sync::Arc;

use scrgb::ScRGB;
use scrgb_tonemapper::{whitepoint::Whitepoint, ScrgbTonemapper};
use selection::Selection;
use thiserror::Error;
use uuid::Uuid;
use vulkan_instance::{texture::Texture, VulkanInstance};
use windows::Win32::Foundation::HWND;
use windows_capture_provider::{display::Display, WindowsCaptureProvider};
use winit::dpi::PhysicalPosition;

use crate::windows_helpers::foreground_window::get_foreground_window;

pub struct ActiveCapture {
    pub display: Display,
    pub texture: Arc<Texture>,
    pub tonemapper: ScrgbTonemapper,
    pub selection: Selection,
    pub formerly_focused_window: HWND,
    pub id: Uuid,
}

impl ActiveCapture {
    pub fn new(
        vk: Arc<VulkanInstance>,
        capture_provider: &mut WindowsCaptureProvider,
    ) -> Result<Self, Error> {
        let id = Uuid::new_v4();
        log::info!(
            "[capture_id]
  {}",
            id
        );
        let formerly_focused_window = get_foreground_window();

        let capture = capture_provider.take_capture()?;
        let texture = Arc::new(Texture::new(&vk, capture.display.size)?);
        let tonemapper = ScrgbTonemapper::new(&vk, texture.image_view.clone(), &capture)?;
        tonemapper.tonemap(&vk)?;

        let selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(capture.display.size[0], capture.display.size[1]),
        );

        let display = capture.display;

        Ok(Self {
            display,
            selection,
            texture,
            tonemapper,
            formerly_focused_window,
            id,
        })
    }

    /// Sets the whitepoint to a new setting.
    pub fn set_whitepoint(
        &mut self,
        vk: &VulkanInstance,
        whitepoint: Whitepoint,
    ) -> Result<(), scrgb_tonemapper::tonemap::Error> {
        self.tonemapper.set_curve_target(whitepoint);
        self.tonemapper.tonemap(vk)?;
        Ok(())
    }

    /// Adjusts the whitepoint of the tonemapper by an amount.
    pub fn adjust_whitepoint(
        &mut self,
        vk: &VulkanInstance,
        amount: ScRGB,
    ) -> Result<(), scrgb_tonemapper::tonemap::Error> {
        self.tonemapper.adjust_whitepoint(amount);
        self.tonemapper.tonemap(vk)?;
        Ok(())
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
