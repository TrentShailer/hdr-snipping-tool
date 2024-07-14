use super::Error;
use crate::active_app::ActiveApp;

impl ActiveApp {
    pub fn redraw(&mut self) -> Result<(), Error> {
        if !self.window.is_visible().unwrap_or(true) {
            return Ok(());
        }

        let Some(capture) = self.active_capture.as_ref() else {
            return Ok(());
        };

        let selection = capture.selection.as_pos_size();

        self.renderer.render(
            &self.vk,
            self.window.clone(),
            selection.0.into(),
            selection.1.into(),
            self.mouse_position.into(),
        )?;

        Ok(())
    }
}
