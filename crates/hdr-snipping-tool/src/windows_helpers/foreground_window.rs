use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow},
};

pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn set_foreground_window(handle: HWND) -> bool {
    if handle.0 == 0 {
        return false;
    }

    let result = unsafe { SetForegroundWindow(handle) };
    result.as_bool()
}
