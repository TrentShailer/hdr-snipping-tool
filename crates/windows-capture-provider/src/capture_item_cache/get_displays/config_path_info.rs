use thiserror::Error;
use tracing::{instrument, Level};
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SDR_WHITE_LEVEL, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    QDC_ONLY_ACTIVE_PATHS,
};
use windows_core::HRESULT;
use windows_result::{Error as WindowsError, Result as WindowsResult};

/// Config for a given display.
pub struct DisplayConfig {
    pub name: [u16; 32],
    pub sdr_reference_white: f32,
}

/// Gets display config using the display config path infos.
#[instrument(skip_all, err)]
pub fn get_display_configs() -> Result<Box<[DisplayConfig]>, Error> {
    let display_config_path_infos = get_display_config_path_infos().map_err(Error::DisplayInfos)?;

    let display_configs = display_config_path_infos
        .iter()
        .map(|path_info| {
            let name = get_device_name(path_info).map_err(Error::Name)?;
            let sdr_reference_white =
                get_sdr_reference_white(path_info).map_err(Error::ReferenceWhite)?;

            Ok(DisplayConfig {
                name,
                sdr_reference_white,
            })
        })
        .collect();

    display_configs
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to get display infos:\n{0}")]
    DisplayInfos(#[source] WindowsError),

    #[error("Failed to get display name:\n{0}")]
    Name(#[source] WindowsError),

    #[error("Failed to get display SDR reference white:\n{0}")]
    ReferenceWhite(#[source] WindowsError),
}

#[instrument(level = Level::DEBUG, skip_all, err)]
fn get_display_config_path_infos() -> WindowsResult<Box<[DISPLAYCONFIG_PATH_INFO]>> {
    let mut path_elements = 0;
    let mut mode_info_elements = 0;
    unsafe {
        let result = GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_elements,
            &mut mode_info_elements,
        )
        .to_hresult();
        windows_result_from_hresult(result, "GetBufferSizes")?;
    };

    let mut paths = vec![Default::default(); path_elements as usize];
    let mut mode_infos = vec![Default::default(); mode_info_elements as usize];
    unsafe {
        let result = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_elements,
            paths.as_mut_ptr(),
            &mut mode_info_elements,
            mode_infos.as_mut_ptr(),
            None,
        )
        .to_hresult();
        windows_result_from_hresult(result, "Query display config")?; // TODO this may error if buffer sizes have changed from the prior call
    };

    Ok(paths.into_boxed_slice())
}

#[instrument(level = Level::DEBUG, skip_all, err)]
fn get_device_name(path_info: &DISPLAYCONFIG_PATH_INFO) -> WindowsResult<[u16; 32]> {
    let mut device_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();

    device_name.header.adapterId = path_info.sourceInfo.adapterId;
    device_name.header.id = path_info.sourceInfo.id;
    device_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
    device_name.header.size = std::mem::size_of_val(&device_name) as u32;

    unsafe {
        let result = HRESULT::from_nt(DisplayConfigGetDeviceInfo(&mut device_name.header));
        windows_result_from_hresult(result, "Device Name")?;
    };

    Ok(device_name.viewGdiDeviceName)
}

#[instrument(level = Level::DEBUG, skip_all, err)]
fn get_sdr_reference_white(path_info: &DISPLAYCONFIG_PATH_INFO) -> WindowsResult<f32> {
    let mut sdr_white_level = DISPLAYCONFIG_SDR_WHITE_LEVEL::default();

    sdr_white_level.header.adapterId = path_info.targetInfo.adapterId;
    sdr_white_level.header.id = path_info.targetInfo.id;
    sdr_white_level.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL;
    sdr_white_level.header.size = std::mem::size_of_val(&sdr_white_level) as u32;

    unsafe {
        let result = HRESULT::from_nt(DisplayConfigGetDeviceInfo(&mut sdr_white_level.header));
        windows_result_from_hresult(result, "SDR White Level")?;
    };

    let sdr_reference_white = sdr_white_level.SDRWhiteLevel as f32 / 1000.0;

    Ok(sdr_reference_white)
}

fn windows_result_from_hresult(hresult: HRESULT, prefix: &str) -> WindowsResult<()> {
    if hresult.is_ok() {
        return Ok(());
    }

    let message = format!("{}:\n{}", prefix, hresult.message());

    Err(windows_result::Error::new(hresult, message))
}
