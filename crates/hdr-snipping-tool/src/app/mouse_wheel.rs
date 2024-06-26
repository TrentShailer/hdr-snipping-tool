use half::f16;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{event::MouseScrollDelta, event_loop::ActiveEventLoop};

use crate::message_box::display_message;

use super::App;

impl App {
    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta, event_loop: &ActiveEventLoop) {
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return,
        };
        let capture = match self.capture.as_mut() {
            Some(v) => v,
            None => return,
        };

        if !app.window.is_visible().unwrap_or(true) {
            return;
        }

        let y_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(_) => return,
        };

        self.scroll += y_delta;

        let result = if self.scroll <= -1.0 {
            self.scroll = 0.0;
            capture.update_tonemapper_settings(app, f16::from_f32(-0.1), f16::ZERO)
        } else if self.scroll >= 1.0 {
            self.scroll = 0.0;
            capture.update_tonemapper_settings(app, f16::from_f32(0.1), f16::ZERO)
        } else {
            Ok(())
        };

        if let Err(e) = result {
            log::error!("{e}");
            display_message("We encountered an error while updaing the tonemapper.\nMore details are in the logs.", MB_ICONERROR);
            event_loop.exit();
        }
    }
}
