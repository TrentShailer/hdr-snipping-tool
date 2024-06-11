use std::time::Instant;

use half::f16;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
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
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return,
        };

        let capture = match self.capture.as_mut() {
            Some(v) => v,
            None => return,
        };

        if event == WindowEvent::Destroyed && app.window_id == window_id {
            self.app = None;
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::Resized(_new_size) => {
                app.renderer.recreate_swapchain = true;
                app.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                self.app = None;
            }
            WindowEvent::RedrawRequested => {
                if !app.window.is_visible().unwrap_or(true) {
                    return;
                }

                if let Err(e) = app.renderer.render(
                    &app.vulkan_instance,
                    app.window.clone(),
                    self.selection.as_ltrb(),
                    self.mouse_position,
                ) {
                    log::error!("{e}");
                    display_message(
                        "We encountered an error during rendering.\nMore details are in the logs.",
                        MB_ICONERROR,
                    );
                    std::process::exit(-1);
                }

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
                            .mouse_pressed(self.mouse_position, app.window.inner_size());
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
                if app.window.is_visible().unwrap_or(true) {
                    if event.repeat == false && event.state == ElementState::Pressed {
                        if event.physical_key == KeyCode::ArrowRight {
                            capture
                                .tonemapper
                                .set_gamma(capture.tonemapper.config.gamma + f16::from_f32(0.025));
                            capture.tonemapper.tonemap(&app.vulkan_instance).unwrap();
                        }
                        if event.physical_key == KeyCode::ArrowLeft {
                            capture
                                .tonemapper
                                .set_gamma(capture.tonemapper.config.gamma - f16::from_f32(0.025));
                            capture.tonemapper.tonemap(&app.vulkan_instance).unwrap();
                        }
                        if event.physical_key == KeyCode::ArrowUp {
                            capture
                                .tonemapper
                                .set_alpha(capture.tonemapper.config.alpha + f16::from_f32(0.1));
                            capture.tonemapper.tonemap(&app.vulkan_instance).unwrap();
                        }
                        if event.physical_key == KeyCode::ArrowDown {
                            capture
                                .tonemapper
                                .set_alpha(capture.tonemapper.config.alpha - f16::from_f32(0.1));
                            capture.tonemapper.tonemap(&app.vulkan_instance).unwrap();
                        }
                    }

                    if event.physical_key == KeyCode::Escape {
                        app.window.set_visible(false);
                        self.capture = None;
                        app.renderer.texture = None;
                        app.renderer.texture_ds = None;
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
                    .mouse_moved(self.mouse_position, app.window.inner_size());
            }
            _ => (),
        }
    }
}
