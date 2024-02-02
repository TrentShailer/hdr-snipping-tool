use glium::{backend::Facade, Display};
use imgui::{Textures, Ui};
use imgui_glium_renderer::Texture;

use crate::{gui::AppEvent, image::Image};

use super::App;

impl App {
    pub fn draw_settings(
        &mut self,
        ui: &mut Ui,
        display: &Display,
        textures: &mut Textures<Texture>,
    ) -> ([f32; 2], [f32; 2]) {
        ui.window("Settings")
            .always_auto_resize(true)
            .collapsible(false)
            .build(|| {
                if ui
                    .input_float("Gamma", &mut self.image.gamma)
                    .step(0.025)
                    .build()
                {
                    self.image.compress_gamma();
                    self.remake_texture(display.get_context(), textures)
                        .unwrap();
                }

                if ui
                    .input_float("Alpha", &mut self.image.alpha)
                    .step(0.025)
                    .build()
                {
                    self.image.compress_gamma();
                    self.remake_texture(display.get_context(), textures)
                        .unwrap();
                };

                if ui.button_with_size("Auto Alpha", [275.0, 25.0]) {
                    self.image.alpha = self.image.calculate_alpha();
                    self.image.compress_gamma();
                    self.remake_texture(display.get_context(), textures)
                        .unwrap();
                }

                ui.spacing();

                if ui.button_with_size("Save and Close", [275.0, 25.0]) {
                    self.save_and_close(display, textures);
                }

                if ui.button_with_size("Exit", [275.0, 25.0]) {
                    self.proxy.send_event(AppEvent::Hide).unwrap();
                    self.image = Image::blank();
                    self.remake_texture(display, textures).unwrap();
                }

                (ui.window_size(), ui.window_pos())
            })
            .unwrap()
    }
}
