use std::time::Instant;

use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{
    event::{MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

use crate::message_box::display_message;

use super::App;

impl App {
    pub(super) fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::Destroyed && self.window_id == Some(window_id) {
            self.window_id = None;
            event_loop.exit();
            return;
        }

        let window = match self.window.as_ref() {
            Some(v) => v,
            None => return,
        };

        let backend = match self.backend.as_mut() {
            Some(v) => v,
            None => return,
        };

        let vulkan_instance = match self.vulkan_instance.as_ref() {
            Some(v) => v,
            None => return,
        };

        match event {
            WindowEvent::Resized(_new_size) => {
                backend.renderer.recreate_swapchain = true;
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                self.window = None;
                self.vulkan_instance = None;
                self.backend = None;
            }
            WindowEvent::RedrawRequested => {
                if !Self::is_visible(&self.window) {
                    return;
                }

                if let Err(e) = backend.renderer.render(
                    &vulkan_instance,
                    window.clone(),
                    self.mouse_position,
                    self.selection.as_ltrb(),
                    window.inner_size(),
                ) {
                    log::error!("{e}");
                    display_message(
                        "We encountered an error during rendering.\nMore details are in the logs.",
                        MB_ICONERROR,
                    );
                    std::process::exit(-1);
                };

                self.last_frame = Instant::now();
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button != MouseButton::Left {
                    return;
                }

                match state {
                    winit::event::ElementState::Pressed => {
                        self.selection
                            .mouse_pressed(self.mouse_position, window.inner_size());
                    }
                    winit::event::ElementState::Released => {
                        let should_save = self.selection.mouse_released();
                        if should_save {
                            self.save_capture();
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if Self::is_visible(&self.window) {
                    if event.physical_key == KeyCode::Escape {
                        window.set_visible(false);
                        backend.renderer.renderpass_capture.capture = None;
                        backend.renderer.renderpass_capture.capture_ds = None;
                        backend.tonemapper.clear();
                    } else if event.physical_key == KeyCode::Enter {
                        self.save_capture();
                    }
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.cast();
                self.selection
                    .mouse_moved(self.mouse_position, window.inner_size());
            }
            _ => (),
        }
    }
}
