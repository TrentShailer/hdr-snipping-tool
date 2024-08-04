use tracing::{info, info_span};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow},
};

pub fn get_foreground_window() -> HWND {
    let _span = info_span!("get_foreground_window").entered();

    let foreground = unsafe { GetForegroundWindow() };

    info!("{:?}", foreground);

    foreground
}

pub fn set_foreground_window(handle: HWND) -> bool {
    let _span = info_span!("set_foreground_window").entered();

    if handle.0.is_null() {
        return false;
    }

    let result = unsafe { SetForegroundWindow(handle).as_bool() };

    info!("{:?}: {}", handle, result);

    result
}
