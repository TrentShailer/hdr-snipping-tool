use winit::event::{DeviceId, ElementState, MouseButton};

use crate::active_app::ActiveApp;

use super::Error;

impl ActiveApp {
    pub fn mouse_input(
        &mut self,
        _device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) -> Result<(), Error> {
        let Some(capture) = self.active_capture.as_mut() else {
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
                    capture.save(&self.vk)?;
                    self.clear_capture()?;
                }
            }
        }

        Ok(())
    }
}
