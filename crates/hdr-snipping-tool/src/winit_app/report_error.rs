use tracing::error;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;

use crate::{
    active_app::{self, take_capture},
    windows_helpers::display_message,
};

use super::window_event;

pub fn report_new_app_error(error: active_app::Error) {
    error!("{error}");
    let message = match error {
        active_app::Error::Window(_) => make_message("creating the window"),
        active_app::Error::TrayIcon(_) => make_message("creating the tray icon"),
        active_app::Error::TrayIconVisible(_) => make_message("changing the tray icon visibility"),
        active_app::Error::VulkanInstance(_) => make_message("creating the Vulkan instance"),
        active_app::Error::Renderer(_) => make_message("creating the renderer"),
        active_app::Error::DxDevices(_) => make_message("creating the DirectX devices"),
        active_app::Error::DisplayCache(_) => make_message("creating the display cache"),
    };
    display_message(&message, MB_ICONERROR);
}

pub fn report_take_capture_error(error: take_capture::Error) {
    error!("{error}");
    let message = match error {
        take_capture::Error::ActiveCapture(_) => make_message("creating the capture"),
        take_capture::Error::LoadCapture(_) => {
            make_message("loading the capture into the renderer")
        }
    };
    display_message(&message, MB_ICONERROR);
}

pub fn report_window_event_error(error: window_event::Error) {
    error!("{error}");
    let message = match error {
        window_event::Error::Render(_) => make_message("rendering"),
        window_event::Error::SaveCapture(_) => make_message("saving the capture"),
        window_event::Error::ClearCapture(_) => make_message("clearning the capture"),
    };
    display_message(&message, MB_ICONERROR);
}

fn make_message(action: &str) -> String {
    format!("We encountered an error while {action}.\nMore details are in the logs.")
}
