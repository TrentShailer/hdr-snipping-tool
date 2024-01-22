use windows::core::Result;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};

pub fn get_display() -> Result<DisplayInfo> {
    let displays = enumerate_displays()?;
    Ok(displays.get(0).unwrap().to_owned())
}

fn enumerate_displays() -> Result<Vec<DisplayInfo>> {
    unsafe {
        let displays = Box::into_raw(Box::default());
        EnumDisplayMonitors(HDC(0), None, Some(enum_monitor), LPARAM(displays as isize)).ok()?;
        Ok(*Box::from_raw(displays))
    }
}

extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<DisplayInfo>));
        let display_info = DisplayInfo::new(monitor).unwrap();
        state.push(display_info);
    }
    true.into()
}

#[derive(Clone)]
pub struct DisplayInfo {
    pub handle: HMONITOR,
    pub display_name: String,
}

impl DisplayInfo {
    pub fn new(monitor_handle: HMONITOR) -> Result<Self> {
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        unsafe {
            GetMonitorInfoW(monitor_handle, &mut info as *mut _ as *mut _).ok()?;
        }

        let display_name = String::from_utf16_lossy(&info.szDevice)
            .trim_matches(char::from(0))
            .to_string();

        Ok(Self {
            handle: monitor_handle,
            display_name,
        })
    }
}
