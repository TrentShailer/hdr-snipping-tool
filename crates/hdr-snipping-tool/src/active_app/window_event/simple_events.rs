use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, Modifiers},
};

use crate::active_app::ActiveApp;

impl ActiveApp {
    pub fn resized(&mut self, _new_size: PhysicalSize<u32>) {
        self.renderer.recreate_swapchain = true;
        self.window.request_redraw();
    }

    pub fn modifiers_changed(&mut self, modifiers: Modifiers) {
        self.keyboard_modifiers = modifiers.state();
    }

    pub fn cursor_moved(&mut self, _device_id: DeviceId, position: PhysicalPosition<f64>) {
        self.mouse_position = position.cast();

        let Some(capture) = self.active_capture.as_mut() else {
            return;
        };

        capture
            .selection
            .mouse_moved(self.mouse_position, self.window.inner_size());
    }
}
