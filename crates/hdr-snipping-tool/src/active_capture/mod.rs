mod capture_image;
mod drop;
pub mod save;
pub mod selection;

use std::sync::Arc;

use ash::{
    vk::{DeviceMemory, Image, ImageView},
    Device,
};
use half::f16;
use scrgb_tonemapper::maximum::{self, Maximum};
use selection::Selection;
use thiserror::Error;
use tracing::{info, info_span};
use vulkan_instance::VulkanInstance;
use windows::Win32::Foundation::HWND;
use windows_capture_provider::{
    capture_item_cache, get_capture::get_capture, hovered, Capture, CaptureItemCache,
    DirectXDevices, Display,
};
use winit::dpi::PhysicalPosition;

use crate::windows_helpers::foreground_window::get_foreground_window;

pub struct ActiveCapture {
    device: Arc<Device>,
    pub capture: Capture,
    pub capture_image: Image,
    pub capture_memory: DeviceMemory,
    pub capture_view: ImageView,
    pub selection: Selection,
    pub formerly_focused_window: HWND,
    pub whitepoint: f32,
}

impl ActiveCapture {
    pub fn new(
        vk: &VulkanInstance,
        maximum_finder: &Maximum,
        dx: &DirectXDevices,
        display_cache: &mut CaptureItemCache,
        hdr_whitepoint: f32,
    ) -> Result<Self, Error> {
        let _span = info_span!("ActiveCapture::new").entered();

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
        let (capture_image, capture_memory, capture_view) = Self::image_from_capture(vk, &capture)?;

        let maximum = maximum_finder.find_maximum(vk, capture_view, display.size)?;
        let maximum = if f16::from_bits(maximum.to_bits() - 1).to_f32()
            == capture.display.sdr_referece_white
        {
            capture.display.sdr_referece_white
        } else {
            maximum.to_f32()
        };

        info!("Maximum: {:.2}", maximum);

        let whitepoint = if maximum > display.sdr_referece_white {
            hdr_whitepoint
        } else {
            display.sdr_referece_white
        };

        info!("Whitepoint: {:.2}", whitepoint);

        let selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(capture.display.size[0], capture.display.size[1]),
        );

        Ok(Self {
            device: vk.device.clone(),
            selection,
            //
            capture,
            capture_image,
            capture_memory,
            capture_view,
            //
            whitepoint,
            formerly_focused_window,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to refresh display cache:\n{0}")]
    RefreshCache(#[from] capture_item_cache::Error),

    #[error("Failed to get hovered display:\n{0}")]
    HoveredDisplay(#[from] hovered::Error),

    #[error("No display is being hovered")]
    NoDisplay,

    #[error("No capture item exists for hovered display: {0:?}")]
    NoCaptureItem(Display),

    #[error("Failed to get capture:\n{0}")]
    GetCapture(#[from] windows_capture_provider::get_capture::Error),

    #[error("Failed to create capture image:\n{0}")]
    CaptureImage(#[from] capture_image::Error),

    #[error("Failed to find maximum capture luminance:\n{0}")]
    Maximum(#[from] maximum::Error),
}
