use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

use crate::active_app;

use super::WinitApp;

impl WinitApp {
    pub fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) -> Result<(), active_app::window_event::Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        if event == WindowEvent::Destroyed && app.window.id() == window_id {
            self.app = None;
            event_loop.exit();
            return Ok(());
        }

        match event {
            WindowEvent::Resized(new_size) => app.resized(new_size),
            WindowEvent::CloseRequested => {
                self.app = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => app.redraw()?,
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => app.keyboard_input(device_id, event, is_synthetic)?,
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => app.mouse_wheel(device_id, delta, phase)?,
            WindowEvent::ModifiersChanged(modifiers) => app.modifiers_changed(modifiers),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => app.mouse_input(device_id, state, button)?,
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => app.cursor_moved(device_id, position),
            _ => (),
        };

        Ok(())
    }
}
