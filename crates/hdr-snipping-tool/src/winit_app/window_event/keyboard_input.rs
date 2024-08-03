use winit::{
    event::{DeviceId, ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::winit_app::WinitApp;

use super::Error;

impl WinitApp {
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

    fn pressed(&mut self, _keycode: KeyCode) -> Result<(), Error> {
        Ok(())
    }

    fn released(&mut self, keycode: KeyCode) -> Result<(), Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        match keycode {
            KeyCode::Escape => self.clear_capture()?,
            KeyCode::Enter => {
                if let Some(capture) = self.capture.as_mut() {
                    capture.save(&app.vk)?;
                    self.clear_capture()?;
                }
            }

            _ => {}
        }

        Ok(())
    }
}
