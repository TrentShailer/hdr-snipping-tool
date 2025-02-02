use std::{fs, io::Read, path::PathBuf};

use global_hotkey::hotkey::Code;
use serde::{Deserialize, Serialize};

use crate::{
    config_dir,
    utilities::failure::{report_and_panic, Failure},
};

const FILE_NAME: &str = "hdr-config.toml";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config {
    pub screenshot_key: Code,
    pub hdr_whitepoint: f32,
}

impl Config {
    pub fn try_load_config() -> Result<Option<Self>, toml::de::Error> {
        let mut file = match fs::File::open(Self::file_path()) {
            Ok(file) => file,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Ok(None);
                }

                report_and_panic(
                    error,
                    "Could not check if an existing configuration file eixsts",
                );
            }
        };

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .report_and_panic("Could not read the existing configuration file");

        let config: Config = toml::from_str(&contents)?;

        Ok(Some(config))
    }

    pub fn save(&self) {
        let toml_string =
            toml::to_string_pretty(self).report_and_panic("Could not save the configuration file");

        fs::write(Self::file_path(), toml_string.as_bytes())
            .report_and_panic("Could not save the configuration file");
    }

    pub fn file_path() -> PathBuf {
        config_dir().join(FILE_NAME)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            screenshot_key: Code::PrintScreen,
            hdr_whitepoint: 6.25,
        }
    }
}
