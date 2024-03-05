use hdr_capture::{HdrCapture, SdrCapture, Tonemapper};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

/// Perform HDR -> SDR tonemapping using the gamma compression algorithm.<br>
/// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
/// a > 0; 0 < γ < 1;<br>
/// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
/// if a < 1 it can decrease the exposure of over exposed parts of the image.
#[derive(Debug, Default)]
pub struct GammaCompressionTonemapper {
    pub alpha: f32,
    pub gamma: f32,
}

impl GammaCompressionTonemapper {
    pub fn new(capture: &HdrCapture, default_gamma: f32) -> Self {
        let gamma = default_gamma;
        let max = capture.get_max_value();
        let alpha = max.powf(-gamma);

        Self { alpha, gamma }
    }

    pub fn calculate_alpha(&self, capture: &HdrCapture) -> f32 {
        let max = capture.get_max_value();
        max.powf(-self.gamma)
    }
}

impl Tonemapper for GammaCompressionTonemapper {
    fn tonemap(&self, hdr_capture: &HdrCapture) -> SdrCapture {
        let capture = hdr_capture
            .data
            .par_iter()
            .enumerate()
            .map(|(index, value)| {
                // ignore alpha channel
                if (index + 1) % 4 == 0 {
                    return (value * F32_255) as u8;
                }

                ((self.alpha * value.powf(self.gamma)) * F32_255) as u8
            })
            .collect();

        SdrCapture {
            data: capture,
            size: hdr_capture.size,
        }
    }

    #[cfg(feature = "imgui")]
    fn render_settings(&mut self, ui: &imgui::Ui, hdr_capture: &HdrCapture) -> bool {
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

const F32_255: f32 = 255.0;
