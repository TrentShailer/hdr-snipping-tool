mod load;
mod old_settings;

use livesplit_hotkey::KeyCode;
use serde::{Deserialize, Serialize};

use self::old_settings::{Pre1_2_0Settings, PreDefaultGammaSettings};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub screenshot_key: KeyCode,
    pub default_gamma: f32,
}

impl Settings {
    fn default() -> Self {
        Self {
            screenshot_key: KeyCode::PrintScreen,
            default_gamma: 0.454545,
        }
    }
}

impl From<Pre1_2_0Settings> for Settings {
    fn from(value: Pre1_2_0Settings) -> Self {
        Self {
            screenshot_key: value.screenshot_key,
            ..Self::default()
        }
    }
}

impl From<PreDefaultGammaSettings> for Settings {
    fn from(value: PreDefaultGammaSettings) -> Self {
        Self {
            screenshot_key: value.screenshot_key,
            ..Self::default()
        }
    }
}
