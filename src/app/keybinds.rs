use glium::Display;
use imgui::{Textures, Ui};
use imgui_glium_renderer::Texture;

use crate::{gui::AppEvent, image::Image};

use super::App;

impl App {
    pub fn handle_keybinds(
        &mut self,
        ui: &mut Ui,
        display: &Display,
        textures: &mut Textures<Texture>,
    ) {
        if ui.is_key_released(imgui::Key::Escape) {
            self.proxy.send_event(AppEvent::Hide).unwrap();
            self.image = Image::blank();
            self.remake_texture(display, textures).unwrap();
            return;
        }

        if ui.is_key_released(imgui::Key::Enter) {
            self.save_and_close(display, textures);
            return;
        }
    }
}
