pub mod save;
pub mod selection;

use crate::{active_app::ActiveApp, windows_helpers::foreground_window::get_foreground_window};
use hdr_capture::{hdr_capture::Error as HdrCaptureError, HdrCapture, Rect};
use selection::Selection;
use thiserror::Error;
use tracing::instrument;
use vulkan_instance::VulkanError;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows_capture_provider::{capture_item_cache, windows_capture, Display, WindowsCapture};

pub struct ActiveCapture {
    pub display: Display,
    pub capture: HdrCapture,
    pub selection: Selection,
    pub formerly_focused_window: HWND,
}

impl ActiveCapture {
    #[instrument("ActiveCapture::new", skip_all, err)]
    pub fn new(app: &mut ActiveApp) -> Result<Self, Error> {
        let ActiveApp {
            vk,
            maximum,
            dx,
            capture_item_cache,
            settings,
            ..
        } = app;

        vk.wake()?;

        let formerly_focused_window = get_foreground_window();

        capture_item_cache.refresh_displays(dx)?;

        let display = match app.capture_item_cache.hovered()? {
            Some(display) => display,
            None => return Err(Error::NoDisplay),
        };

        let windows_capture = WindowsCapture::take_capture(app.dx.clone(), display)?;

        let hdr_capture = HdrCapture::import_windows_capture(
            vk.clone(),
            maximum,
            &windows_capture,
            settings.hdr_whitepoint,
        )?;

        let selection = Selection::new(Rect {
            start: [0, 0],
            end: hdr_capture.size,
        });

        unsafe {
            if !windows_capture.handle.is_invalid() {
                CloseHandle(*windows_capture.handle)?;
            }
        }

        Ok(Self {
            selection,
            capture: hdr_capture,
            formerly_focused_window,
            display: windows_capture.display,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed while accessing the capture item cache:\n{0}")]
    HoveredDisplay(#[from] capture_item_cache::Error),

    #[error("No display is being hovered")]
    NoDisplay,

    #[error("Failed to take capture:\n{0}")]
    TakeCapture(#[from] windows_capture::Error),

    #[error("Failed to import capture to vulkan:\n{0}")]
    ImportCapture(#[from] HdrCaptureError),

    #[error("Failed to close handle:\n{0}")]
    CloseHandle(#[from] windows_result::Error),

    #[error("Failed during vulkan call:\n{0}")]
    Vulkan(#[from] VulkanError),
}
