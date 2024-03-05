use gamma_compression_tonemapper::GammaCompressionTonemapper;

use super::ImguiSettings;

impl ImguiSettings for GammaCompressionTonemapper {
    fn render_settings(&mut self, ui: &imgui::Ui, hdr_capture: &hdr_capture::HdrCapture) -> bool {
        let mut value_changed = false;
        if ui.input_float("Gamma", &mut self.gamma).step(0.025).build() {
            value_changed = true;
        }

        if ui.input_float("Alpha", &mut self.alpha).step(0.025).build() {
            value_changed = true;
        };

        if ui.button_with_size("Auto Alpha", [275.0, 25.0]) {
            self.alpha = self.calculate_alpha(hdr_capture);
            value_changed = true;
        }

        value_changed
    }
}
