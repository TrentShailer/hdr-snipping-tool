use half::f16;

use crate::Tonemapper;

impl Tonemapper {
    pub fn set_gamma(&mut self, gamma: f16) {
        self.config.gamma = gamma;
    }

    pub fn set_alpha(&mut self, alpha: f16) {
        self.config.alpha = alpha;
    }
}
