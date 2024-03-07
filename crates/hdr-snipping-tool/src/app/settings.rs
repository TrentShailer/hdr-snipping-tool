mod gamma_compression_tonemapper;

use error_trace::{ErrorTrace, OptionExt, ResultExt};
use glow::Texture;
use hdr_capture::{HdrCapture, LogicalBounds, Tonemapper};
use imgui::{Textures, Ui};
use winit::dpi::{LogicalPosition, LogicalSize};

use super::App;

impl<T: Tonemapper + ImguiSettings> App<T> {
    /// Draws the settings window and returns its position and size
    pub fn draw_settings(
        &mut self,
        ui: &Ui,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<LogicalBounds, ErrorTrace> {
        ui.window("Settings")
            .always_auto_resize(true)
            .collapsible(false)
            .build(|| {
                if self.tonemapper.render_settings(ui, &self.capture.hdr) {
                    self.tonemap();
                    if let Err(e) = self.rebuild_texture(textures, gl).track() {
                        log::error!("{}", e.to_string());
                    };
                }

                ui.spacing();

                if ui.button_with_size("Save and Close", [275.0, 25.0]) {
                    self.save().track()?;
                    self.close().track()?;
                }

                if ui.button_with_size("Exit", [275.0, 25.0]) {
                    self.close().track()?;
                }

                let pos: LogicalPosition<f32> = ui.window_pos().into();
                let size: LogicalSize<f32> = ui.window_size().into();

                Ok(LogicalBounds::from((pos, size)))
            })
            .track()?
    }
}

pub trait ImguiSettings {
    fn render_settings(&mut self, ui: &imgui::Ui, hdr_capture: &HdrCapture) -> bool;
}
