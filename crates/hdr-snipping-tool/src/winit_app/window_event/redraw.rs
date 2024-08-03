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

        let selection = capture.selection.as_pos_size();

        app.renderer.render(
            &app.vk,
            app.window.clone(),
            selection.0.into(),
            selection.1.into(),
            self.mouse_position.into(),
            false,
        )?;

        Ok(())
    }
}
