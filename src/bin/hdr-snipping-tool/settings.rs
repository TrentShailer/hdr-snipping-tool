use std::{fs, io::Read};

use livesplit_hotkey::KeyCode;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub version: String, // Version stored in settings to allow for handling any future breaking changes to existing settings files.
    pub screenshot_key: KeyCode,
}

impl Settings {
    pub fn load() -> Result<Self, Whatever> {
        let file = fs::File::open(SETTINGS_FILE_PATH);

        if file
            .as_ref()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let settings = Self::default();
            settings
                .save()
                .whatever_context("Failed to save settings")?;

            return Ok(settings);
        }

        let mut file = file.whatever_context("Failed to open settings file.")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .whatever_context("Failed to read settings file.")?;

        let mut settings: Settings =
            toml::from_str(&contents).whatever_context("Failed to deserialize settings.")?;

        if settings.version != Self::default().version {
            settings.version = Self::default().version;
            settings
                .save()
                .whatever_context("Failed to save settings")?;
        }

        Ok(settings)
    }

    fn default() -> Self {
        Self {
            version: String::from("1.1.2"),
            screenshot_key: KeyCode::PrintScreen,
        }
    }

    fn save(&self) -> Result<(), Whatever> {
        let toml_string =
            toml::to_string_pretty(self).whatever_context("Failed to serialize settings.")?;

        fs::write(SETTINGS_FILE_PATH, toml_string.as_bytes())
            .whatever_context("Failed to write settings file.")?;
        Ok(())
    }
}
