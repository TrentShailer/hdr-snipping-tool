use thiserror::Error;
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::active_capture::{self, ActiveCapture};

use super::ActiveApp;

impl ActiveApp {
    pub fn take_capture(&mut self) -> Result<(), Error> {
        let active_capture = ActiveCapture::new(&self.vk, &mut self.capture_provider)?;

        let size: PhysicalSize<u32> = active_capture.capture.display.size.into();
        let _ = self.window.request_inner_size(size);

        let position: PhysicalPosition<i32> = active_capture.capture.display.position.into();
        self.window.set_outer_position(position);

        self.renderer.parameters.set_whitepoint(
            &self.vk,
            &mut self.renderer.glyph_cache,
            active_capture.tonemapper.whitepoint,
        )?;

        self.renderer
            .capture
            .load_capture(&self.vk, active_capture.texture.clone())?;

        self.window.set_visible(true);
        self.window.focus_window();

        self.active_capture = Some(active_capture);

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create active capture:\n{0}")]
    ActiveCapture(#[from] active_capture::Error),

    #[error("Failed to load capture into renderer:\n{0}")]
    LoadCapture(#[from] vulkan_renderer::capture::load::Error),

    #[error("Failed to update UI text:\n{0}")]
    UpdateText(#[from] vulkan_renderer::text::set_text::Error),
}
