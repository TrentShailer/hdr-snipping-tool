use glium::glutin::dpi::{LogicalPosition, LogicalSize};
use windows::core::Result;
use windows::Win32::Foundation::{BOOL, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub fn get_display() -> Result<DisplayInfo> {
    let displays = enumerate_displays()?;
    let display = get_hovered_display(displays);
    Ok(display)
}

fn get_hovered_display(displays: Vec<DisplayInfo>) -> DisplayInfo {
    let mut pos: POINT = Default::default();
    unsafe {
        GetCursorPos(&mut pos).unwrap();
    };

    let display = displays
        .into_iter()
        .find(|display| point_in_rect(pos, display.rect));

    display.unwrap()
}

fn point_in_rect(point: POINT, rect: RECT) -> bool {
    point.x >= rect.left && point.x <= rect.right && point.y >= rect.top && point.y <= rect.bottom
}

fn enumerate_displays() -> Result<Vec<DisplayInfo>> {
    unsafe {
        let displays = Box::into_raw(Box::default());
        EnumDisplayMonitors(HDC(0), None, Some(enum_monitor), LPARAM(displays as isize)).ok()?;
        Ok(*Box::from_raw(displays))
    }
}

extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, rect: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        let rect = rect.read();
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<DisplayInfo>));
        let display_info = DisplayInfo::new(monitor, rect).unwrap();
        state.push(display_info);
    }
    true.into()
}

#[derive(Clone, Debug)]
pub struct DisplayInfo {
    pub handle: HMONITOR,
    pub display_name: String,
    pub rect: RECT,
}

impl DisplayInfo {
    pub fn new(monitor_handle: HMONITOR, rect: RECT) -> Result<Self> {
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
            rect,
        })
    }

    pub fn get_position(&self) -> LogicalPosition<i32> {
        LogicalPosition::new(self.rect.left, self.rect.top)
    }

    pub fn get_size(&self) -> LogicalSize<i32> {
        LogicalSize::new(
            self.rect.right - self.rect.left,
            self.rect.bottom - self.rect.top,
        )
    }
}
