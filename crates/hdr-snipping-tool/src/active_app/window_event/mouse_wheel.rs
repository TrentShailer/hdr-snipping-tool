use scrgb::ScRGB;
use winit::event::{DeviceId, MouseScrollDelta, TouchPhase};

use crate::active_app::ActiveApp;

use super::Error;

impl ActiveApp {
    pub fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
    ) -> Result<(), Error> {
        if self.active_capture.is_none() {
            return Ok(());
        }

        let y_delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(_) => return Ok(()),
        };

        self.scroll += y_delta;

        if self.scroll.abs() < 1.0 {
            return Ok(());
        }

        let whitepoint_change_amount = if self.scroll < 0.0 {
            ScRGB::from_nits(-10.0)
        } else {
            ScRGB::from_nits(10.0)
        };

        self.scroll = 0.0;

        self.adjust_whitepoint(whitepoint_change_amount)?;

        Ok(())
    }
}
