mod config_path_info;
mod descriptors;

use config_path_info::get_display_configs;
use descriptors::get_output_descriptors;

use thiserror::Error;
use tracing::{info, info_span};
use windows_result::Error as WindowsError;

use crate::{DirectXDevices, Display};

/// Gets the currently attached displays.
pub fn get_displays(devices: &DirectXDevices) -> Result<Box<[Display]>, Error> {
    let _span = info_span!("get_displays").entered();

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

            let found_display = Display::new(
                descriptor.Monitor,
                descriptor.DesktopCoordinates,
                config.sdr_reference_white,
                descriptor.MaxLuminance,
            );

            info!("{}", found_display);

            Some(found_display)
        })
        .collect();

    Ok(displays)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get display output descriptors:\n{0}")]
    GetDescriptors(#[source] WindowsError),

    #[error("Failed to get display configs:\n{0}")]
    GetDisplayConfigs(#[from] config_path_info::Error),
}
