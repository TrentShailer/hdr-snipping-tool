use std::{fs, io::Read};

use snafu::{ResultExt, Whatever};

use super::{pre_1_2_0_settings::Pre1_2_0Settings, Settings};

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

impl Settings {
    pub fn migrate(contents: &str) -> Self {
        if let Ok(pre_1_2_0_settings) = toml::from_str::<Pre1_2_0Settings>(contents) {
            return Settings::from(pre_1_2_0_settings);
        }

        Self::default()
    }

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

        let settings: Settings = match toml::from_str(&contents) {
            Ok(v) => v,
            Err(_) => Self::migrate(&contents),
        };

        settings
            .save()
            .whatever_context("Failed to save settings")?;

        Ok(settings)
    }

    fn save(&self) -> Result<(), Whatever> {
        let toml_string =
            toml::to_string_pretty(self).whatever_context("Failed to serialize settings.")?;

        fs::write(SETTINGS_FILE_PATH, toml_string.as_bytes())
            .whatever_context("Failed to write settings file.")?;
        Ok(())
    }
}
