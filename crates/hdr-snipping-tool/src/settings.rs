mod load;

use global_hotkey::hotkey::Code;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub screenshot_key: Code,
    pub default_gamma: f32,
}

impl Settings {
    fn default() -> Self {
        Self {
            screenshot_key: Code::PrintScreen,
            default_gamma: 0.48,
        }
    }
}
