use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{event::MouseScrollDelta, event_loop::ActiveEventLoop};

use crate::message_box::display_message;

use super::App;

impl App {
    pub fn mouse_wheel(&mut self, delta: MouseScrollDelta, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };
        let Some(capture) = self.capture.as_mut() else {
            return;
        };

        if !app.window.is_visible().unwrap_or(true) {
            return;
        }

        let y_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(_) => return,
        };

        self.scroll += y_delta;

        if self.scroll.abs() < 1.0 {
            return;
        }

        let mut alpha_increment = if self.scroll < 0.0 { -0.01 } else { 0.01 };

        if self.keyboard_modifiers.shift_key() {
            alpha_increment *= 10.0;
        }

        self.scroll = 0.0;

        if let Err(e) = capture.update_tonemapper_settings(app, alpha_increment, 0.0) {
            log::error!("{e}");
            display_message("We encountered an error while updaing the tonemapper.\nMore details are in the logs.", MB_ICONERROR);
            event_loop.exit();
        }
    }
}
