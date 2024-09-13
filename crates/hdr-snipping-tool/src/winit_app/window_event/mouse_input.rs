use winit::event::{DeviceId, ElementState, MouseButton};

use crate::winit_app::WinitApp;

use super::Error;

impl WinitApp {
    pub fn mouse_input(
        &mut self,
        _device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) -> Result<(), Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        let Some(capture) = self.capture.as_mut() else {
            return Ok(());
        };

        if button != MouseButton::Left {
            return Ok(());
        }

        match state {
            winit::event::ElementState::Pressed => {
                capture.selection.start_selection(self.mouse_position);
            }
            winit::event::ElementState::Released => {
                let should_save = capture.selection.end_selection();
                if should_save {
                    capture.save(app)?;
                    self.clear_capture()?;
                }
            }
        }

        Ok(())
    }
}
