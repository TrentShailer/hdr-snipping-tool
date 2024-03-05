mod gamma_compression_tonemapper;

use hdr_capture::{HdrCapture, LogicalBounds, Tonemapper};
use imgui::Ui;
use winit::dpi::{LogicalPosition, LogicalSize};

use super::{app_event::AppEvent, App};

impl<T: Tonemapper + ImguiSettings> App<T> {
    /// Draws the settings window and returns its position and size
    pub fn draw_settings(&mut self, ui: &Ui) -> LogicalBounds {
        ui.window("Settings")
            .always_auto_resize(true)
            .collapsible(false)
            .build(|| {
                if self.tonemapper.render_settings(ui, &self.capture.hdr) {
                    self.event_queue
                        .append(&mut [AppEvent::Tonemap, AppEvent::RebuildTexture].into());
                }

                ui.spacing();

                if ui.button_with_size("Save and Close", [275.0, 25.0]) {
                    self.event_queue
                        .append(&mut [AppEvent::Save, AppEvent::Close].into());
                }

                if ui.button_with_size("Exit", [275.0, 25.0]) {
                    self.event_queue.push_back(AppEvent::Close);
                }

                let pos: LogicalPosition<f32> = ui.window_pos().into();
                let size: LogicalSize<f32> = ui.window_size().into();

                LogicalBounds::from((pos, size))
            })
            .unwrap()
    }
}

pub trait ImguiSettings {
    fn render_settings(&mut self, ui: &imgui::Ui, hdr_capture: &HdrCapture) -> bool;
}
