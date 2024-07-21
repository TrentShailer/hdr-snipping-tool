use std::sync::Arc;

use thiserror::Error;
use winit::{
    dpi::PhysicalSize,
    error::OsError,
    event_loop::ActiveEventLoop,
    platform::windows::IconExtWindows,
    window::{BadIcon, Icon, Window},
};

use super::WinitApp;

impl WinitApp {
    pub(super) fn create_window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>, Error> {
        let window_icon = Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))?;

        let window_attributes = Window::default_attributes()
            .with_title("HDR Snipping Tool")
            .with_window_icon(Some(window_icon))
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            .with_active(false)
            .with_visible(false);

        let window = Arc::from(event_loop.create_window(window_attributes)?);

        Ok(window)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get icon:\n{0}")]
    BadIcon(#[from] BadIcon),

    #[error("Failed to create window:\n{0}")]
    CreateWindow(#[from] OsError),
}
