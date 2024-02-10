mod gamma_compression;

use crate::{HdrCapture, SdrCapture};

pub use gamma_compression::GammaCompressionTonemapper;
use imgui::Ui;

pub trait ToneMapper {
    fn tonemap(&self, hdr_capture: &HdrCapture) -> SdrCapture;

    /// Function to handle rendering ui for the settings of the tone mapper
    /// returns true if tonemapping should be applied
    fn render_settings(&mut self, ui: &Ui, hdr_capture: &HdrCapture) -> bool;
}
