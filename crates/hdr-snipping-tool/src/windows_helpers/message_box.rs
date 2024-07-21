use windows::{
    core::{h, HSTRING},
    Win32::UI::WindowsAndMessaging::{MessageBoxW, MESSAGEBOX_STYLE},
};

pub fn display_message(message: &str, style: MESSAGEBOX_STYLE) {
    unsafe {
        let message = HSTRING::from(message);

        MessageBoxW(None, &message, h!("HDR Snipping Tool"), style);
    }
}
