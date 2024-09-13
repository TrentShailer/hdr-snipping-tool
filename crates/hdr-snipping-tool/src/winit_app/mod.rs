mod clear_capture;
mod report_error;
mod window_event;

use std::process::Command;

use report_error::make_message;
use tracing::error;
use tray_icon::menu::MenuEvent;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::WindowId,
};

use crate::{
    active_app::ActiveApp, active_capture::ActiveCapture, project_directory, settings::Settings,
    windows_helpers::display_message,
};

pub struct WinitApp {
    pub app: Option<ActiveApp>,
    pub capture: Option<ActiveCapture>,
    pub settings: Settings,
    pub mouse_position: PhysicalPosition<u32>,
}

impl WinitApp {
    pub fn new(settings: Settings) -> Self {
        Self {
            app: None,
            capture: None,
            settings,
            mouse_position: PhysicalPosition::default(),
        }
    }
}

impl ApplicationHandler<()> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let app = match ActiveApp::new(event_loop, self.settings) {
            Ok(app) => app,
            Err(error) => {
                error!("{error}");
                display_message(&make_message("setting up the app"), MB_ICONERROR);
                event_loop.exit();
                return;
            }
        };

        self.app = Some(app);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        let Some(app) = self.app.as_mut() else { return };

        let active_capture = match ActiveCapture::new(app) {
            Ok(capture) => capture,
            Err(error) => {
                error!("{error}");
                display_message(&make_message("taking the capture"), MB_ICONERROR);
                event_loop.exit();
                return;
            }
        };

        let size: PhysicalSize<u32> = active_capture.capture.size.into();
        let _ = app.window.request_inner_size(size);

        let position: PhysicalPosition<i32> = active_capture.display.position.into();
        app.window.set_outer_position(position);

        if let Err(error) = app.renderer.load_capture(&active_capture.capture) {
            error!("{error}");
            display_message(&make_message("taking the capture"), MB_ICONERROR);
            event_loop.exit();
            return;
        };

        app.window.set_visible(true);
        app.window.focus_window();

        self.capture = Some(active_capture);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_ref() else { return };

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "0" => {
                    if let Err(e) = Command::new("explorer").arg(project_directory()).spawn() {
                        error!("{e}");
                        display_message(&make_message("opening file explorer"), MB_ICONERROR);
                        event_loop.exit();
                    }
                }
                "1" => event_loop.exit(),
                _ => {}
            }
        }

        app.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Err(error) = self.on_window_event(event_loop, window_id, event) {
            error!("{error}");
            display_message(&make_message("handling window events"), MB_ICONERROR);
            event_loop.exit();
        }
    }
}
