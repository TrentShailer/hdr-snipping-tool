use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;

use crate::{windows_helpers::display_message, AppError};

pub fn report_app_error(error: AppError) {
    log::error!("{error}");

    let message = match error {
        AppError::OnlyInstance(_) => {
            make_message("while checking that there were no other instances running")
        }
        AppError::GraphicsCaptureSupport(_) => {
            make_message("while checking if your device supports graphics capture")
        }
        AppError::NoCaptureSupport => "Your devices does not support graphics capture.".to_string(),
        AppError::LoadSettings(_) => make_message("while loading your settings"),
        AppError::SaveSettings(_) => make_message("while saving your settings"),
        AppError::EventLoop(_) => make_message("in the event loop"),
        AppError::Hotkey(_) => make_message("while registering your hotkey"),
    };

    display_message(&message, MB_ICONERROR);
}

fn make_message(action: &str) -> String {
    format!("We encountered an error {action}.\nMore details are in the logs.")
}
