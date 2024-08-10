use thiserror::Error;
use tracing::info_span;
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::active_capture::{self, ActiveCapture};

use super::ActiveApp;

impl ActiveApp {
    pub fn take_capture(&mut self, hdr_whitepoint: f32) -> Result<ActiveCapture, Error> {
        let _span = info_span!("ActiveApp::take_capture").entered();

        let active_capture =
            ActiveCapture::new(&self.vk, &self.dx, &mut self.display_cache, hdr_whitepoint)?;

        let size: PhysicalSize<u32> = active_capture.display.size.into();
        let _ = self.window.request_inner_size(size);

        let position: PhysicalPosition<i32> = active_capture.display.position.into();
        self.window.set_outer_position(position);

        self.renderer.capture.load_capture(
            &self.vk,
            active_capture.capture_image.clone(),
            active_capture.whitepoint,
        )?;

        self.window.set_visible(true);
        self.window.focus_window();

        Ok(active_capture)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create active capture:\n{0}")]
    ActiveCapture(#[from] active_capture::Error),

    #[error("Failed to load capture into renderer:\n{0}")]
    LoadCapture(#[from] vulkan_renderer::capture::load::Error),
}
