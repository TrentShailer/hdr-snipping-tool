use livesplit_hotkey::KeyCode;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pre1_2_0Settings {
    pub version: String,
    pub screenshot_key: KeyCode,
}
