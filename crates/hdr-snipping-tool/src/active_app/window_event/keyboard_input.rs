use scrgb::ScRGB;
use winit::{
    event::{DeviceId, ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::active_app::ActiveApp;

use super::Error;

impl ActiveApp {
    pub fn keyboard_input(
        &mut self,
        _device_id: DeviceId,
        event: KeyEvent,
        _is_synthetic: bool,
    ) -> Result<(), Error> {
        let keycode: KeyCode = match event.physical_key {
            PhysicalKey::Code(code) => code,
            PhysicalKey::Unidentified(_) => return Ok(()),
        };

        match event.state {
            ElementState::Pressed => self.pressed(keycode)?,
            ElementState::Released => self.released(keycode)?,
        }

        Ok(())
    }

    fn pressed(&mut self, keycode: KeyCode) -> Result<(), Error> {
        match keycode {
            KeyCode::Escape => self.clear_capture()?,
            KeyCode::Enter => {
                if let Some(capture) = self.active_capture.as_mut() {
                    capture.save(&self.vk)?;
                    self.clear_capture()?;
                }
            }

            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                let amount = if keycode == KeyCode::ArrowUp {
                    ScRGB::from_nits(10.0)
                } else {
                    ScRGB::from_nits(-10.0)
                };
                self.adjust_whitepoint(amount)?;
            }

            _ => {}
        }

        Ok(())
    }

    fn released(&mut self, _keycode: KeyCode) -> Result<(), Error> {
        Ok(())
    }
}
