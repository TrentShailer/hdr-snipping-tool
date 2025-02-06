use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SDR_WHITE_LEVEL, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS,
    },
    Graphics::Dxgi::DXGI_OUTPUT_DESC1,
};
use windows_core::HRESULT;

use crate::{LabelledWinResult, WinError};

use super::{Error, Monitor};

impl Monitor {
    pub(super) fn get_sdr_white(descriptor: DXGI_OUTPUT_DESC1) -> Result<f32, Error> {
        let mut path_elements = 0;
        let mut mode_info_elements = 0;
        unsafe {
            let result = GetDisplayConfigBufferSizes(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_elements,
                &mut mode_info_elements,
            );

            if result.is_err() {
                return Err(WinError::from_win32(result, "GetDisplayConfigBufferSizes").into());
            }
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
            );

            if result.is_err() {
                return Err(WinError::from_win32(result, "QueryDisplayConfig").into());
            }
        };

        let matching_path = {
            let names = paths
                .iter()
                .map(|path| match get_device_name(path) {
                    Ok(name) => Ok(name),
                    Err(e) => Err(e),
                })
                .collect::<LabelledWinResult<Vec<_>>>()?;

            let maybe_matching_path_index =
                names.iter().position(|name| *name == descriptor.DeviceName);

            let matching_path_index = match maybe_matching_path_index {
                Some(index) => index,
                None => return Err(Error::MonitorsMismatch),
            };

            paths[matching_path_index]
        };

        let sdr_white = get_sdr_white(&matching_path)?;

        Ok(sdr_white)
    }
}

/// Get device name, matches the descriptor DeviceName
fn get_device_name(path_info: &DISPLAYCONFIG_PATH_INFO) -> LabelledWinResult<[u16; 32]> {
    let mut config = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();

    config.header.adapterId = path_info.sourceInfo.adapterId;
    config.header.id = path_info.sourceInfo.id;
    config.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
    config.header.size = core::mem::size_of_val(&config) as u32;

    unsafe {
        let result = DisplayConfigGetDeviceInfo(&mut config.header);

        let hresult = HRESULT::from_nt(result);
        if hresult.is_err() {
            return Err(WinError::from_hresult(
                hresult,
                "DisplayConfigGetDeviceInfo",
            ));
        }
    };

    Ok(config.viewGdiDeviceName)
}

fn get_sdr_white(path_info: &DISPLAYCONFIG_PATH_INFO) -> LabelledWinResult<f32> {
    let mut config = DISPLAYCONFIG_SDR_WHITE_LEVEL::default();

    config.header.adapterId = path_info.targetInfo.adapterId;
    config.header.id = path_info.targetInfo.id;
    config.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL;
    config.header.size = core::mem::size_of_val(&config) as u32;

    unsafe {
        let result = DisplayConfigGetDeviceInfo(&mut config.header);

        let hresult = HRESULT::from_nt(result);
        if hresult.is_err() {
            return Err(WinError::from_hresult(
                hresult,
                "DisplayConfigGetDeviceInfo",
            ));
        }
    };

    let sdr_white = config.SDRWhiteLevel as f32 / 1000.0;

    Ok(sdr_white)
}
