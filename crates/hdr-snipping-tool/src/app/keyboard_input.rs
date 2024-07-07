use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{
    event::{ElementState, KeyEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::message_box::display_message;

use super::App;

impl App {
    pub fn keyboard_input(&mut self, event: KeyEvent, event_loop: &ActiveEventLoop) {
        if self.app.is_none() || self.capture.is_none() {
            return;
        }

        let keycode: KeyCode = match event.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => return,
        };

        match event.state {
            ElementState::Pressed => self.pressed(keycode, event_loop),
            ElementState::Released => self.released(keycode, event_loop),
        }
    }

    fn pressed(&mut self, keycode: KeyCode, event_loop: &ActiveEventLoop) {
        match keycode {
            KeyCode::Escape => self.clear_capture(),
            KeyCode::Enter => {
                if let Err(e) = self.save_capture() {
                    log::error!("{e}");
                    display_message(
						"We encountered an error while saving the capture.\nMore details are in the logs.",
						MB_ICONERROR,
					);
                    event_loop.exit();
                }
            }
            KeyCode::ArrowRight | KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                self.adjust_tonemap_settings(keycode, event_loop)
            }
            _ => {}
        }
    }

    fn released(&mut self, keycode: KeyCode, _event_loop: &ActiveEventLoop) {
        match keycode {
            _ => {}
        }
    }

    fn adjust_tonemap_settings(&mut self, keycode: KeyCode, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };
        let Some(capture) = self.capture.as_mut() else {
            return;
        };

        let mut alpha_increment = 0.0;
        let mut gamma_increment = 0.0;

        match keycode {
            KeyCode::ArrowRight => gamma_increment = 0.01,
            KeyCode::ArrowLeft => gamma_increment = -0.01,
            KeyCode::ArrowUp => alpha_increment = 0.01,
            KeyCode::ArrowDown => alpha_increment = -0.01,
            _ => return,
        };

        if self.keyboard_modifiers.shift_key() {
            alpha_increment *= 10.0;
            gamma_increment *= 10.0;
        }

        let update_result =
            capture.update_tonemapper_settings(app, alpha_increment, gamma_increment);

        if let Err(e) = update_result {
            log::error!("{e}");
            display_message("We encountered an error while updaing the tonemapper.\nMore details are in the logs.", MB_ICONERROR);
            event_loop.exit();
            return;
        }
    }
}
