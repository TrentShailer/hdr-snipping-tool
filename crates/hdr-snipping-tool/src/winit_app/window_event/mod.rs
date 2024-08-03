mod keyboard_input;
mod mouse_input;
mod redraw;

use thiserror::Error;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::WindowId};

use crate::active_capture;

use super::{clear_capture, WinitApp};

impl WinitApp {
    pub fn on_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) -> Result<(), Error> {
        let Some(app) = self.app.as_mut() else {
            return Ok(());
        };

        if event == WindowEvent::Destroyed && app.window.id() == window_id {
            self.app = None;
            self.capture = None;
            event_loop.exit();
            return Ok(());
        }

        match event {
            WindowEvent::Resized(_) => {
                app.renderer.recreate_swapchain = true;
                app.window.request_redraw();
            }

            WindowEvent::CloseRequested => {
                self.app = None;
                self.capture = None;
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => self.redraw()?,

            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => self.mouse_input(device_id, state, button)?,

            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.cast();

                let Some(capture) = self.capture.as_mut() else {
                    return Ok(());
                };

                capture.selection.update_selection(self.mouse_position);
            }

            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => self.keyboard_input(device_id, event, is_synthetic)?,

            _ => (),
        };

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to render:\n{0}")]
    Render(#[from] vulkan_renderer::renderer::render::Error),

    #[error("Failed to save capture:\n{0}")]
    SaveCapture(#[from] active_capture::save::Error),

    #[error("Failed to clear capture:\n{0}")]
    ClearCapture(#[from] clear_capture::Error),
}
