mod config_path_info;
mod descriptors;

use std::time::Instant;

use config_path_info::get_display_configs;
use descriptors::get_output_descriptors;

use thiserror::Error;
use windows_result::Error as WindowsError;

use crate::{DirectXDevices, Display};

/// Gets the currently attached displays.
pub fn get_displays(devices: &DirectXDevices) -> Result<Box<[Display]>, Error> {
    let start = Instant::now();

    // Descriptors provide most of the information about the display.
    let descriptors = get_output_descriptors(devices).map_err(Error::GetDescriptors)?;
    // Config Path Infos provide the sdr_reference_white
    let display_configs = get_display_configs()?;

    // The descriptors and display_configs need to be matched up.
    // This is done by using the device name.
    let displays: Box<[Display]> = descriptors
        .iter()
        .filter_map(|descriptor| {
            let config = display_configs
                .iter()
                .find(|config| config.name == descriptor.DeviceName)?;

            Some(Display::new(
                descriptor.Monitor,
                descriptor.DesktopCoordinates,
                config.sdr_reference_white,
            ))
        })
        .collect();

    log::debug!(
        "[get_displays]{}
  [TIMING] {}ms",
        displays.iter().fold(String::new(), |acc, display| format!(
            "{}\n  {}",
            acc, display
        )),
        start.elapsed().as_millis()
    );

    Ok(displays)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get display output descriptors:\n{0}")]
    GetDescriptors(#[source] WindowsError),

    #[error("Failed to get display configs:\n{0}")]
    GetDisplayConfigs(#[from] config_path_info::Error),
}
