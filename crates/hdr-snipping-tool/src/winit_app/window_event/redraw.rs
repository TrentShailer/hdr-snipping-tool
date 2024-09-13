use hdr_capture::Rect;

use crate::winit_app::WinitApp;

use super::Error;

impl WinitApp {
    pub fn redraw(&mut self) -> Result<(), Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        if !app.window.is_visible().unwrap_or(true) {
            return Ok(());
        }

        let selection_rect = if let Some(capture) = self.capture.as_ref() {
            capture.selection.rect
        } else {
            Rect {
                start: [0, 0],
                end: [4096, 4096],
            }
        };

        app.renderer
            .render(&app.window, self.mouse_position.into(), selection_rect)?;

        Ok(())
    }
}
