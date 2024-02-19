mod load;
mod pre_1_2_0_settings;

use livesplit_hotkey::KeyCode;
use serde::{Deserialize, Serialize};

use self::pre_1_2_0_settings::Pre1_2_0Settings;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub screenshot_key: KeyCode,
}

impl Settings {
    fn default() -> Self {
        Self {
            screenshot_key: KeyCode::PrintScreen,
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
