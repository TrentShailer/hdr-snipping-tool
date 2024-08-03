pub mod save;
pub mod selection;

use std::sync::Arc;

use scrgb_tonemapper::{tonemap, tonemap_output::TonemapOutput};
use selection::Selection;
use thiserror::Error;
use uuid::Uuid;
use vulkan_instance::VulkanInstance;
use windows::Win32::Foundation::HWND;
use windows_capture_provider::{
    display_cache, get_capture::get_capture, hovered, DirectXDevices, Display, DisplayCache,
};
use winit::dpi::PhysicalPosition;

use crate::windows_helpers::foreground_window::get_foreground_window;

pub struct ActiveCapture {
    pub display: Display,
    pub tonemap_output: Arc<TonemapOutput>,
    pub selection: Selection,
    pub formerly_focused_window: HWND,
    pub id: Uuid,
}

impl ActiveCapture {
    pub fn new(
        vk: &VulkanInstance,
        dx: &DirectXDevices,
        display_cache: &mut DisplayCache,
        hdr_whitepoint: f32,
    ) -> Result<Self, Error> {
        let id = Uuid::new_v4();
        log::info!(
            "[capture_id]
  {}",
            id
        );

        let formerly_focused_window = get_foreground_window();

        display_cache.refresh(dx)?;

        let display = match display_cache.hovered()? {
            Some(display) => display,
            None => return Err(Error::NoDisplay),
        };

        let capture_item = match display_cache
            .capture_items
            .get(&(display.handle.0 as isize))
        {
            Some(capture_item) => capture_item,
            None => return Err(Error::NoCaptureItem(display)),
        };

        let capture = get_capture(dx, &display, capture_item)?;
        let tonemap_output = Arc::new(tonemap(vk, &capture, hdr_whitepoint)?);

        let selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(capture.display.size[0], capture.display.size[1]),
        );

        let display = capture.display;

        Ok(Self {
            display,
            selection,
            tonemap_output,
            formerly_focused_window,
            id,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to refresh display cache:\n{0}")]
    RefreshCache(#[from] display_cache::Error),

    #[error("Failed to get hovered display:\n{0}")]
    HoveredDisplay(#[from] hovered::Error),

    #[error("No display is being hovered")]
    NoDisplay,

    #[error("No capture item exists for hovered display: {0:?}")]
    NoCaptureItem(Display),

    #[error("Failed to get capture:\n{0}")]
    GetCapture(#[from] windows_capture_provider::get_capture::Error),

    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] scrgb_tonemapper::Error),
}
