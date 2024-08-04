use thiserror::Error;
use tracing::info_span;

use crate::windows_helpers::foreground_window::set_foreground_window;

use super::WinitApp;

impl WinitApp {
    pub fn clear_capture(&mut self) -> Result<(), Error> {
        let _span = info_span!("WinitApp::clear_capture").entered();

        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        app.renderer.capture.unload_capture();

        if let Some(capture) = self.capture.as_ref() {
            let selection = capture.selection.as_pos_size();

            app.renderer.render(
                &app.vk,
                app.window.clone(),
                selection.0.into(),
                selection.1.into(),
                self.mouse_position.into(),
                true,
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
    Render(#[from] vulkan_renderer::renderer::render::Error),
}
