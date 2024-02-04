use std::{fs, io::Read};

use anyhow::Context;
use livesplit_hotkey::KeyCode;
use serde::{Deserialize, Serialize};

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub version: String, // Version stored in settings to allow for handling any future breaking changes to existing settings files.
    pub screenshot_key: KeyCode,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let file = fs::File::open(SETTINGS_FILE_PATH);

        if file
            .as_ref()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let settings = Self::default();
            settings.save().context("Failed to save settings")?;

            return Ok(settings);
        }

        let mut file = file.context("Failed to open settings file.")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .context("Failed to read settings file.")?;

        let mut settings: Settings =
            toml::from_str(&contents).context("Failed to deserialize settings.")?;

        if settings.version != Self::default().version {
            settings.version = Self::default().version;
            settings.save().context("Failed to save settings")?;
        }

        Ok(settings)
    }

    fn default() -> Self {
        Self {
            version: String::from("1.1.2"),
            screenshot_key: KeyCode::PrintScreen,
        }
    }

    fn save(&self) -> anyhow::Result<()> {
        let toml_string = toml::to_string_pretty(self).context("Failed to serialize settings.")?;

        fs::write(SETTINGS_FILE_PATH, toml_string.as_bytes())
            .context("Failed to write settings file.")?;
        Ok(())
    }
}
