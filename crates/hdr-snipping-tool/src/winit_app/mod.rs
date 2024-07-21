mod resumed;
mod window_event;

use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    active_app::{self, take_capture, ActiveApp},
    windows_helpers::display_message,
};

pub struct WinitApp {
    pub app: Option<ActiveApp>,
}

impl WinitApp {
    pub fn new() -> Self {
        Self { app: None }
    }
}

impl ApplicationHandler<()> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.on_resume(event_loop) {
            log::error!("{e}");
            let message = match e {
                resumed::Error::Window(_) =>
                    "We encountered an error while creating the window.\nMore details are in the logs.",
                resumed::Error::TrayIcon(_) =>
                    "We encountered an error while creating the tray icon.\nMore details are in the logs.",
                resumed::Error::TrayIconVisible(_) =>
                    "We encountered an error while changing the tray icon visibility.\nMore details are in the logs.",
                resumed::Error::VulkanInstance(_) =>
                    "We encountered an error while creating the Vulkan instance.\nMore details are in the logs.",
                resumed::Error::Renderer(_) =>
                    "We encountered an error while creating the renderer.\nMore details are in the logs.",
				resumed::Error::CaptureProvider(_) =>
					"We encountered an error while creating the capture provider.\nMore details are in the logs.",
            };
            display_message(message, MB_ICONERROR);
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        let Some(app) = self.app.as_mut() else { return };
        if let Err(e) = app.take_capture() {
            log::error!("{e}");
            let message = match e {
				take_capture::Error::ActiveCapture(_) => "We encountered an error while getting the capture.\nMore details are in the logs.",
				take_capture::Error::LoadCapture(_) => "We encountered an error while loading the capture.\nMore details are in the logs.",
				take_capture::Error::UpdateText(_) => "We encoutnered an error while updating the text.\nMore details are in the logs.",
            };
            display_message(message, MB_ICONERROR);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };

        app.handle_tray_icon(event_loop);

        app.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Err(e) = self.on_window_event(event_loop, window_id, event) {
            log::error!("{e}");
            let message = match e {
                active_app::window_event::Error::Render(_) => "We encountered an error while rendering.\nMore details are in the logs.",
				active_app::window_event::Error::AdjustTonemapSettings(_) => "We encountered an error while adjusting the tonemapping settings.\nMore details are in the logs.",
				active_app::window_event::Error::SaveCapture(_) => "We encountered an error while saving the capture.\nMore details are in the logs.",
				active_app::window_event::Error::ClearCapture(_) => "We encountered an error while clearing the capture.\nMore details are in the logs."
            };
            display_message(message, MB_ICONERROR);
            event_loop.exit();
        };
    }
}
