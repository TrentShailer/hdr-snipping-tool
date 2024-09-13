use thiserror::Error;
use tracing::{info_span, instrument};

use crate::windows_helpers::foreground_window::set_foreground_window;

use super::WinitApp;

impl WinitApp {
    #[instrument("WinitApp::clear_capture", skip_all, err)]
    pub fn clear_capture(&mut self) -> Result<(), Error> {
        let _span = info_span!("WinitApp::clear_capture").entered();

        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        app.renderer.unload_capture();

        if let Some(capture) = self.capture.as_ref() {
            app.renderer.render(
                &app.window,
                self.mouse_position.into(),
                capture.selection.rect,
            )?;

            set_foreground_window(capture.formerly_focused_window);
        }

        self.capture = None;
        app.window.set_visible(false);

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to clear renderer:\n{0}")]
    Render(#[from] vulkan_renderer::Error),
}
