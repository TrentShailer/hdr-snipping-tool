use std::cmp::Ordering;

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
    pub default_gamma: f32,
}

impl GammaCompressionTonemapper {
    pub fn new(gamma: f32) -> Self {
        Self {
            alpha: 0.5,
            gamma,
            default_gamma: gamma,
        }
    }

    pub fn calculate_alpha(&self, capture: &HdrCapture) -> f32 {
        let max = Self::get_max_value(capture);
        max.powf(-self.gamma)
    }

    fn get_max_value(capture: &HdrCapture) -> &f32 {
        capture
            .data
            .par_iter()
            .enumerate()
            .max_by(|(a_index, a), (b_index, b)| {
                let is_a_alpha = (a_index + 1) % 4 == 0;
                let is_b_alpha = (b_index + 1) % 4 == 0;

                if is_a_alpha && !is_b_alpha {
                    return Ordering::Less;
                }

                if !is_a_alpha && is_b_alpha {
                    return Ordering::Greater;
                }

                if is_a_alpha && is_b_alpha {
                    return Ordering::Equal;
                }

                a.total_cmp(b)
            })
            .unwrap()
            .1
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

    fn reset_settings(&mut self, hdr_capture: &HdrCapture) {
        self.gamma = self.default_gamma;

        let max = Self::get_max_value(hdr_capture);
        let alpha = max.powf(-self.gamma);

        self.alpha = alpha;
    }
}

const F32_255: f32 = 255.0;
