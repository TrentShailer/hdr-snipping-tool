use imgui::Ui;

use crate::gui::AppEvent;

use super::App;

impl App {
    pub fn handle_keybinds(&mut self, ui: &mut Ui) {
        if ui.is_key_released(imgui::Key::Escape) {
            self.proxy.send_event(AppEvent::Hide).unwrap();
            return;
        }

        if ui.is_key_released(imgui::Key::Enter) {
            self.save_and_close();
            return;
        }
    }
}
