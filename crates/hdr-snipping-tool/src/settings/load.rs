use std::{fs, io::Read};

use error_trace::{ErrorTrace, ResultExt};

use super::Settings;

const SETTINGS_FILE_PATH: &str = "./hdr-config.toml";

impl Settings {
    pub fn migrate(_contents: &str) -> Self {
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
