mod clear_capture;
mod report_error;
mod window_event;

use report_error::{report_new_app_error, report_take_capture_error, report_window_event_error};
use winit::{
    application::ApplicationHandler, dpi::PhysicalPosition, event::WindowEvent,
    event_loop::ActiveEventLoop, window::WindowId,
};

use crate::{active_app::ActiveApp, active_capture::ActiveCapture, settings::Settings};

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
        let app = match ActiveApp::new(event_loop) {
            Ok(app) => app,
            Err(error) => {
                report_new_app_error(error);
                event_loop.exit();
                return;
            }
        };

        self.app = Some(app);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        let Some(app) = self.app.as_mut() else { return };

        let capture = match app.take_capture(self.settings.hdr_whitepoint) {
            Ok(capture) => capture,
            Err(error) => {
                report_take_capture_error(error);
                event_loop.exit();
                return;
            }
        };

        self.capture = Some(capture);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_ref() else { return };

        app.handle_tray_icon(event_loop);
        app.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Err(error) = self.on_window_event(event_loop, window_id, event) {
            report_window_event_error(error);
            event_loop.exit();
        }
    }
}
