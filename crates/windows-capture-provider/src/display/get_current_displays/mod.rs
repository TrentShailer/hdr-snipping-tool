mod display_configs;

use display_configs::get_display_configs;
use thiserror::Error;
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter1, IDXGIOutput6, DXGI_OUTPUT_DESC1};
use windows_core::Interface;
use windows_result::{Error as WindowsError, Result as WindowsResult};

use super::Display;

/// Gets the current displays by enumerating the `IDXGIAdapter1`'s output descriptors and matching them
/// with their `DISPLAYCONFIG_PATH_INFO` containing their SDR reference white.
pub fn get_current_displays(dxgi_adapter: &IDXGIAdapter1) -> Result<Box<[Display]>, Error> {
    let descriptors = get_output_descriptors(dxgi_adapter).map_err(Error::GetDescriptors)?;
    let display_configs = get_display_configs()?;

    let displays = descriptors
        .into_iter()
        .filter_map(|descriptor| {
            let maybe_config = display_configs
                .iter()
                .find(|config| config.name == descriptor.DeviceName);
            let Some(config) = maybe_config else {
                return None;
            };

            Some(Display::from_desc1(descriptor, config.sdr_reference_white))
        })
        .collect();

    Ok(displays)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get display output descriptors:\n{0}")]
    GetDescriptors(#[source] WindowsError),

    #[error("Failed to get display configs:\n{0}")]
    GetDisplayConfigs(#[from] display_configs::Error),
}

fn get_output_descriptors(dxgi_adapter: &IDXGIAdapter1) -> WindowsResult<Box<[DXGI_OUTPUT_DESC1]>> {
    let mut output_descs = vec![];
    let mut i = 0;

    while let Ok(output) = unsafe { dxgi_adapter.EnumOutputs(i) } {
        let output_6: IDXGIOutput6 = output.cast()?;

        let mut output_desc_1: DXGI_OUTPUT_DESC1 = Default::default();
        unsafe {
            output_6.GetDesc1(&mut output_desc_1)?;
        };

        output_descs.push(output_desc_1);

        i += 1;
    }

    Ok(output_descs.into_boxed_slice())
}
