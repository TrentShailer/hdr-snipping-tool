use std::{
    fs,
    io::{self, Read},
};

use global_hotkey::hotkey::Code;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub screenshot_key: Code,
    pub default_gamma: f32,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("Failed to open settings file:\n{0}")]
    OpenFile(#[source] io::Error),

    #[error("Failed to save settings file:\n{0}")]
    SaveFile(#[from] SaveError),

    #[error("Failed to read settings file:\n{0}")]
    ReadFile(#[source] io::Error),

    #[error("Failed to deserialize settings:\n{0}")]
    Deserialize(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("Failed to serialize settings:\n{0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("Failed to write file:\n{0}")]
    Write(#[from] io::Error),
}

impl Settings {
    pub fn load_or_create() -> Result<Self, LoadError> {
        let file = fs::File::open(SETTINGS_FILE_PATH);

        if file
            .as_ref()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let settings = Self::default();
            settings.save()?;

            return Ok(settings);
        }

        let mut file = file.map_err(LoadError::OpenFile)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(LoadError::ReadFile)?;

        let settings: Settings = toml::from_str(&contents)?;

        Ok(settings)
    }

    pub fn save(&self) -> Result<(), SaveError> {
        let toml_string = toml::to_string_pretty(self)?;

        fs::write(SETTINGS_FILE_PATH, toml_string.as_bytes())?;
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            screenshot_key: Code::PrintScreen,
            default_gamma: 0.5,
        }
    }
}
