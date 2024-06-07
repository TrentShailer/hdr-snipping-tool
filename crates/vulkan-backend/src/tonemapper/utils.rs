use half::f16;

use super::Tonemapper;

impl Tonemapper {
    pub fn clear(&mut self) {
        self.active_tonemapper = None;
    }

    pub fn set_alpha(&mut self, new_alpha: f16) -> bool {
        let active_tonemapper = match self.active_tonemapper.as_mut() {
            Some(v) => v,
            None => return false,
        };

        active_tonemapper.alpha = new_alpha;
        true
    }

    pub fn set_gamma(&mut self, new_gamma: f16) -> bool {
        let active_tonemapper = match self.active_tonemapper.as_mut() {
            Some(v) => v,
            None => return false,
        };

        active_tonemapper.gamma = new_gamma;
        true
    }
}
