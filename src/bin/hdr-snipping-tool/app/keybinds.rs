use imgui::Ui;

use super::{app_event::AppEvent, App};

impl App {
    pub fn handle_keybinds(&mut self, ui: &Ui) {
        if ui.is_key_released(imgui::Key::Escape) {
            self.event_queue.push_back(AppEvent::Close);
        }

        if ui.is_key_released(imgui::Key::Enter) {
            self.event_queue
                .append(&mut [AppEvent::Save, AppEvent::Close].into());
        }
    }
}
