use snafu::{ResultExt, Snafu};
use windows::Win32::{
    Foundation::{BOOL, LPARAM, POINT, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
    UI::WindowsAndMessaging::GetCursorPos,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::DisplayInfo;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("The mouse is not inside any display"))]
    NoHoveredDisplay,
    #[snafu(display("Windows API call '{call}' returned an error."))]
    WindowsApi {
        source: windows::core::Error,
        call: &'static str,
    },
}

pub struct WindowsDisplay {
    pub handle: HMONITOR,
    pub info: DisplayInfo,
}

impl WindowsDisplay {
    pub fn new(handle: HMONITOR, info: DisplayInfo) -> Self {
        Self { handle, info }
    }

    pub fn contains(&self, point: PhysicalPosition<i32>) -> bool {
        let position = self.info.position;
        let size = self.info.size;

        point.x >= position.x
            && point.y >= position.y
            && point.x <= position.x + size.width as i32
            && point.y <= position.y + size.height as i32
    }
}

/// Get the display that the mouse is currently hovering over
pub fn get_hovered_display() -> Result<WindowsDisplay, Error> {
    let displays = get_all_displays()?;

    let mouse_pos = unsafe {
        let mut pos: POINT = Default::default();
        GetCursorPos(&mut pos).context(WindowsApiSnafu {
            call: "GetCursorPos",
        })?;
        PhysicalPosition::new(pos.x, pos.y)
    };

    match displays
        .into_iter()
        .find(|display| display.contains(mouse_pos))
    {
        Some(display) => Ok(display),
        None => Err(Error::NoHoveredDisplay),
    }
}

fn get_all_displays() -> Result<Vec<WindowsDisplay>, Error> {
    unsafe {
        let displays = Box::into_raw(Box::default());

        EnumDisplayMonitors(HDC(0), None, Some(enum_monitor), LPARAM(displays as isize))
            .ok()
            .context(WindowsApiSnafu {
                call: "EnumDisplayMonitors",
            })?;

        Ok(*Box::from_raw(displays))
    }
}

extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, rect: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        let rect = rect.read();
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<WindowsDisplay>));

        let width = (rect.right - rect.left).unsigned_abs();
        let height = (rect.bottom - rect.top).unsigned_abs();

        let display_size = PhysicalSize::new(width, height);
        let display_position = PhysicalPosition::new(rect.left, rect.top);

        let display_info = DisplayInfo::new(display_position, display_size);
        state.push(WindowsDisplay::new(monitor, display_info));
    }
    true.into()
}
