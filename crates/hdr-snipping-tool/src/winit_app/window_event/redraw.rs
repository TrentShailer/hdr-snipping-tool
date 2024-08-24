use crate::winit_app::WinitApp;

use super::Error;

impl WinitApp {
    pub fn redraw(&mut self) -> Result<(), Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        let Some(capture) = self.capture.as_mut() else {
            return Ok(());
        };

        if !app.window.is_visible().unwrap_or(true) {
            return Ok(());
        }

        app.renderer.render(
            &app.window,
            self.mouse_position.into(),
            capture.selection.rect,
        )?;

        Ok(())
    }
}
