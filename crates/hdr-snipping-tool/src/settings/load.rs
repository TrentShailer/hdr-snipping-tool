use std::{fs, io::Read};

use error_trace::{ErrorTrace, ResultExt};

use super::{
    old_settings::{Pre1_2_0Settings, PreDefaultGammaSettings},
    Settings,
};

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

impl Settings {
    pub fn migrate(contents: &str) -> Self {
        if let Ok(old_settings) = toml::from_str::<Pre1_2_0Settings>(contents) {
            return Settings::from(old_settings);
        }

        if let Ok(old_settings) = toml::from_str::<PreDefaultGammaSettings>(contents) {
            return Settings::from(old_settings);
        }

        Self::default()
    }

    pub fn load() -> Result<Self, ErrorTrace> {
        let file = fs::File::open(SETTINGS_FILE_PATH);

        if file
            .as_ref()
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let settings = Self::default();
            settings.save().track()?;

            return Ok(settings);
        }

        let mut file = file.track()?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).track()?;

        let settings: Settings = match toml::from_str(&contents) {
            Ok(v) => v,
            Err(_) => Self::migrate(&contents),
        };

        settings.save().track()?;

        Ok(settings)
    }

    fn save(&self) -> Result<(), ErrorTrace> {
        let toml_string = toml::to_string_pretty(self).track()?;

        fs::write(SETTINGS_FILE_PATH, toml_string.as_bytes()).track()?;
        Ok(())
    }
}
