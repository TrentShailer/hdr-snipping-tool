use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
        DISPLAYCONFIG_SDR_WHITE_LEVEL, DISPLAYCONFIG_SOURCE_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    },
    Graphics::Dxgi::{IDXGIAdapter1, IDXGIOutput6, DXGI_OUTPUT_DESC1},
};
use windows_core::{Interface, HRESULT};

use crate::display::Display;

#[derive(Clone, Copy, PartialEq)]
pub struct DisplayConfig {
    pub name: [u16; 32],
    pub sdr_whitepoint: f32,
    pub sdr_whitepoint_nits: f32,
}

pub fn refresh_displays(
    dxgi_adapter: &IDXGIAdapter1,
    displays: &mut Vec<Display>,
) -> Result<(), windows_result::Error> {
    let output_descs = get_output_descs(dxgi_adapter)?;
    let display_configs = get_display_configs()?;

    let monitor_handles = output_descs
        .iter()
        .map(|desc| desc.Monitor)
        .collect::<Box<[_]>>();

    // filter all of the displays that are no longer present
    displays.retain(|display| monitor_handles.contains(&display.handle));

    // The two sets of display information can be linked using the name.
    for desc in output_descs {
        let maybe_config = display_configs
            .iter()
            .find(|config| config.name == desc.DeviceName);

        let Some(config) = maybe_config else {
            continue; // TODO how to handle this case.
        };

        // check if a display exists with matching HMONITOR, update it
        let found_display = displays.iter_mut().find(|d| **d == desc.Monitor);
        if let Some(found_display) = found_display {
            found_display.update(&desc, config);
            continue;
        }

        // else create a new one
        let display = Display::new(&desc, config)?;
        displays.push(display);
    }

    Ok(())
}

fn get_output_descs(
    dxgi_adapter: &IDXGIAdapter1,
) -> windows_result::Result<Vec<DXGI_OUTPUT_DESC1>> {
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

    Ok(output_descs)
}

fn get_display_configs() -> windows_result::Result<Vec<DisplayConfig>> {
    let mut output = vec![];

    let mut path_elements = 0;
    let mut mode_info_elements = 0;
    unsafe {
        let result = GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_elements,
            &mut mode_info_elements,
        )
        .to_hresult();
        result_from_hresult(result, "GetBufferSizes")?;
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
        result_from_hresult(result, "Query display config")?; // TODO this may error if buffer sizes have changed from the prior call
    };

    for path in paths {
        let mut device_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        device_name.header.adapterId = path.sourceInfo.adapterId;
        device_name.header.id = path.sourceInfo.id;
        device_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        device_name.header.size = std::mem::size_of_val(&device_name) as u32;
        unsafe {
            let result = HRESULT::from_nt(DisplayConfigGetDeviceInfo(&mut device_name.header));
            result_from_hresult(result, "Device Name")?;
        };
        let name = device_name.viewGdiDeviceName;

        let mut sdr_white_level = DISPLAYCONFIG_SDR_WHITE_LEVEL::default();
        sdr_white_level.header.adapterId = path.targetInfo.adapterId;
        sdr_white_level.header.id = path.targetInfo.id;
        sdr_white_level.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL;
        sdr_white_level.header.size = std::mem::size_of_val(&sdr_white_level) as u32;
        unsafe {
            let result = HRESULT::from_nt(DisplayConfigGetDeviceInfo(&mut sdr_white_level.header));
            result_from_hresult(result, "SDR White Level")?;
        };

        let sdr_whitepoint = sdr_white_level.SDRWhiteLevel as f32 / 1000.0 * 80.0;
        let sdr_whitepoint_nits = sdr_white_level.SDRWhiteLevel as f32 / 1000.0;

        let display_config = DisplayConfig {
            name,
            sdr_whitepoint,
            sdr_whitepoint_nits,
        };
        output.push(display_config);
    }

    Ok(output)
}

fn result_from_hresult(hresult: HRESULT, prefix: &str) -> windows_result::Result<()> {
    if hresult.is_ok() {
        return Ok(());
    }

    let message = format!("{}:\n{}", prefix, hresult.message());

    Err(windows_result::Error::new(hresult, message))
}
