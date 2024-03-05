use crate::{HdrCapture, SdrCapture};

pub trait Tonemapper {
    fn tonemap(&self, hdr_capture: &HdrCapture) -> SdrCapture;
    #[cfg(feature = "imgui-settings-renderer")]
    fn render_settings(&mut self, ui: &imgui::Ui, hdr_capture: &HdrCapture) -> bool;
}
