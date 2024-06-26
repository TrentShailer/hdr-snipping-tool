use half::f16;
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

        if event.state == ElementState::Pressed {
            let mut alpha_increment = f16::ZERO;
            let mut gamma_increment = f16::ZERO;

            let keycode: KeyCode = match event.physical_key {
                PhysicalKey::Code(code) => code,
                PhysicalKey::Unidentified(_) => return,
            };

            match keycode {
                KeyCode::ArrowRight => gamma_increment = f16::from_f32(0.02),
                KeyCode::ArrowLeft => gamma_increment = f16::from_f32(-0.02),
                KeyCode::ArrowUp => alpha_increment = f16::from_f32(0.1),
                KeyCode::ArrowDown => alpha_increment = f16::from_f32(-0.1),
                _ => {}
            };

            if alpha_increment != f16::ZERO || gamma_increment != f16::ZERO {
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

        if event.physical_key == KeyCode::Escape {
            self.clear_capture();
        } else if event.physical_key == KeyCode::Enter {
            let result = self.save_capture();

            if let Err(e) = result {
                log::error!("{e}");
                display_message(
                "We encountered an error while saving the capture.\nMore details are in the logs.",
                MB_ICONERROR,
            );
                event_loop.exit();
            }
        }
    }
}
