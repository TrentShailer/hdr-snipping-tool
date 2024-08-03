use std::time::Instant;

use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow},
};

pub fn get_foreground_window() -> HWND {
    let start = Instant::now();

    let foreground = unsafe { GetForegroundWindow() };

    log::debug!(
        "[get_foreground_window]
  {:?}
  [TIMING] {}ms",
        foreground,
        start.elapsed().as_millis()
    );

    foreground
}

pub fn set_foreground_window(handle: HWND) -> bool {
    let start = Instant::now();

    if handle.0.is_null() {
        return false;
    }

    let result = unsafe { SetForegroundWindow(handle).as_bool() };

    log::debug!(
        "[set_foreground_window]
  {:?}: {}
  [TIMING] {}ms",
        handle,
        result,
        start.elapsed().as_millis()
    );

    result
}
