use windows::Win32::{
    Foundation::{BOOL, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::display::Display;

pub fn refresh_displays(
    old_displays: &mut [Display],
) -> Result<Vec<Display>, windows_result::Error> {
    let mut current_displays = get_all_displays()?;

    for display in current_displays.iter_mut() {
        // if the display exists in old_displays, use the created capture item
        let found_display = old_displays.iter_mut().find(|d| *d == display);
        if let Some(found_display) = found_display {
            display.capture_item = found_display.capture_item.take();
            continue;
        }

        display.create_capture_item()?;
    }

    Ok(current_displays)
}

fn get_all_displays() -> windows_result::Result<Vec<Display>> {
    let displays = unsafe {
        let displays_ptr = Box::into_raw(Box::default());

        EnumDisplayMonitors(
            HDC(0),
            None,
            Some(enum_monitor),
            LPARAM(displays_ptr as isize),
        )
        .ok()?;

        *Box::from_raw(displays_ptr)
    };

    Ok(displays)
}

// We pass enum_monster a pointer to our list we want to populate as the LPARAM
extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, rect: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        let rect = rect.read();
        let displays_ref: &mut Vec<Display> =
            Box::leak(Box::from_raw(state.0 as *mut Vec<Display>));

        let width = (rect.right - rect.left).unsigned_abs();
        let height = (rect.bottom - rect.top).unsigned_abs();

        let display_size = PhysicalSize::new(width, height);
        let display_position = PhysicalPosition::new(rect.left, rect.top);

        displays_ref.push(Display::new(monitor, display_position, display_size));
    }
    true.into()
}
