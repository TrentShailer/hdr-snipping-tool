use error_trace::{ErrorTrace, ResultExt};
use hdr_capture::Tonemapper;
use imgui::Ui;

use super::{settings::ImguiSettings, App};

impl<T: Tonemapper + ImguiSettings> App<T> {
    pub fn handle_keybinds(&mut self, ui: &Ui) -> Result<(), ErrorTrace> {
        if ui.is_key_released(imgui::Key::Escape) {
            self.close().track()?;
        }

        if ui.is_key_released(imgui::Key::Enter) {
            self.save().track()?;
            self.close().track()?;
        }

        Ok(())
    }
}
