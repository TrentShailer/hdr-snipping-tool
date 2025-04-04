use std::{process::Command, sync::Arc};

use tracing::{debug, info, warn};
use tray_icon::menu::MenuEvent;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{
    application::capture_taker::Whitepoint,
    config::Config,
    config_dir, screenshot_dir,
    selection::SelectionState,
    utilities::{failure::Failure, windows_helpers::set_foreground_window},
};

use super::{
    Application, TRAY_CONFIG_ID, TRAY_QUIT_ID, TRAY_SCREENSHOT_ID, capture::Capture,
    capture_taker::CaptureProgress,
};

pub enum WindowMessage {
    TakeCapture,
    CaptureProgress(CaptureProgress),
}

pub struct WinitApp {
    proxy: EventLoopProxy<WindowMessage>,
    config: Config,
    application: Option<Application>,
    mouse_position: PhysicalPosition<f32>,
}

impl WinitApp {
    pub fn new(config: Config, proxy: EventLoopProxy<WindowMessage>) -> Self {
        Self {
            proxy,
            config,
            application: None,
            mouse_position: PhysicalPosition::default(),
        }
    }
}

impl ApplicationHandler<WindowMessage> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let application = Application::new(event_loop, self.config);
        self.application = Some(application);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WindowMessage) {
        if event_loop.exiting() {
            return;
        }

        let Some(application) = self.application.as_mut() else {
            return;
        };

        match event {
            WindowMessage::TakeCapture => {
                if application.capture.is_some() {
                    return;
                }

                let proxy = self.proxy.clone();
                if application.capture_taker.take_capture(proxy).is_err() {
                    event_loop.exit();
                    warn!("Exiting: CaptureTaker::take_capture returned Err");
                };
            }

            WindowMessage::CaptureProgress(message) => {
                debug!("Capture Progress: {}", message);

                match message {
                    CaptureProgress::FoundMonitor(monitor) => {
                        // Create capture for that monitor
                        let capture = Capture::new(
                            Arc::clone(&application.vulkan),
                            Arc::clone(&application.capture_taker),
                            monitor,
                        );

                        // Reset renderer state
                        {
                            application.renderer.set_mouse_position(self.mouse_position);
                            application.renderer.set_selection(capture.selection);
                        }

                        // Update application
                        application.capture = Some(capture);
                        application.update_window();

                        // Request redraw
                        if application.renderer.render().is_err() {
                            event_loop.exit();
                            warn!("Exiting: Renderer::render returned Err");
                        }
                    }

                    CaptureProgress::CaptureTaken(windows_capture) => {
                        if let Some(capture) = application.capture.as_mut() {
                            capture.windows_capture = Some(windows_capture);

                            // Update window
                            {
                                application.window.set_visible(true);
                                application.window.focus_window();

                                application.update_window();
                            }

                            if application.renderer.render().is_err() {
                                event_loop.exit();
                                warn!("Exiting: Renderer::render returned Err");
                            }
                        }
                    }

                    CaptureProgress::Imported(hdr_image) => {
                        if let Some(capture) = application.capture.as_mut() {
                            capture.hdr_capture = Some(hdr_image);

                            application.renderer.set_hdr_capture(capture.hdr_capture);

                            if application.renderer.render().is_err() {
                                event_loop.exit();
                                warn!("Exiting: Renderer::render returned Err");
                            }
                        }
                    }

                    CaptureProgress::FoundWhitepoint(whitepoint) => {
                        if let Some(capture) = application.capture.as_mut() {
                            // Set the max brightness based on if the content is SDR or HDR
                            let whitepoint = match whitepoint {
                                Whitepoint::Sdr(whitepoint) => {
                                    application
                                        .renderer
                                        .set_max_brightness(capture.monitor.sdr_white);

                                    debug!("Preview max brightness: {}", capture.monitor.sdr_white);

                                    whitepoint
                                }

                                Whitepoint::Hdr(whitepoint) => {
                                    application
                                        .renderer
                                        .set_max_brightness(capture.monitor.max_brightness);

                                    debug!(
                                        "Preview max brightness: {}",
                                        capture.monitor.max_brightness
                                    );

                                    whitepoint
                                }
                            };

                            // Set the whitepoint
                            capture.whitepoint = whitepoint;
                            application.renderer.set_whitepoint(capture.whitepoint);
                            if application.renderer.render().is_err() {
                                event_loop.exit();
                                warn!("Exiting: Renderer::render returned Err");
                            }
                        }
                    }

                    CaptureProgress::Failed => {
                        application.window.set_visible(false);

                        if let Some(capture) = application.capture.take() {
                            set_foreground_window(capture.formerly_focused_window.0);
                        };
                    }
                }
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.application.take();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }

        if self.application.is_none() {
            return;
        }

        // Handle tray events
        'tray: {
            let Ok(event) = MenuEvent::receiver().try_recv() else {
                break 'tray;
            };

            match event.id.0.as_str() {
                TRAY_SCREENSHOT_ID => {
                    Command::new("explorer")
                        .arg(screenshot_dir())
                        .spawn()
                        .report_and_panic("Could not open Windows Exporer")
                        .wait()
                        .report_and_panic("Could not open Windows Exporer");
                }

                TRAY_CONFIG_ID => {
                    Command::new("explorer")
                        .arg(config_dir())
                        .spawn()
                        .report_and_panic("Could not open Windows Exporer")
                        .wait()
                        .report_and_panic("Could not open Windows Exporer");
                }

                TRAY_QUIT_ID => {
                    event_loop.exit();
                }

                _ => {}
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if event_loop.exiting() {
            return;
        }

        let Some(application) = self.application.as_mut() else {
            return;
        };

        if event == WindowEvent::Destroyed && application.window.id() == window_id {
            event_loop.exit();
            return;
        }

        // Handle window events
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(_) => {
                if application.renderer.resize().is_err() {
                    event_loop.exit();
                    warn!("Exiting: Renderer::resize returned Err");
                }
                application.window.request_redraw();
            }

            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                let Some(capture) = application.capture.take() else {
                    return;
                };

                // Close
                if is_close_event(&event) {
                    set_foreground_window(capture.formerly_focused_window.0);
                    application.window.set_visible(false);
                    application.renderer.set_hdr_capture(None);
                    if application.renderer.render().is_err() {
                        event_loop.exit();
                        warn!("Exiting: Renderer::render returned Err");
                    }

                    info!("Cancelled screenshot");

                    return;
                }

                // Save
                if is_save_event(&event) {
                    debug!("Requested save");
                    set_foreground_window(capture.formerly_focused_window.0);
                    application.window.set_visible(false);
                    if application.capture_saver.save(capture).is_err() {
                        event_loop.exit();
                        warn!("Exiting: CaptureSaver::save returned Err");
                    }
                    application.renderer.set_hdr_capture(None);
                    if application.renderer.render().is_err() {
                        event_loop.exit();
                        warn!("Exiting: Renderer::render returned Err");
                    }

                    return;
                }

                application.capture = Some(capture);
            }

            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.cast();

                // Update renderer
                application.renderer.set_mouse_position(self.mouse_position);

                // Update selection
                if let Some(capture) = application.capture.as_mut() {
                    capture.selection.update_selection(self.mouse_position);
                    application.renderer.set_selection(capture.selection);
                    application.renderer.set_selection(capture.selection);
                }

                application.window.request_redraw();
            }

            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button != MouseButton::Left {
                    return;
                }

                let Some(mut capture) = application.capture.take() else {
                    return;
                };

                match state {
                    // Start selection
                    ElementState::Pressed => {
                        capture.selection.start_selection(self.mouse_position);
                        application.renderer.set_selection(capture.selection);
                        application.window.request_redraw();
                    }

                    // Save
                    ElementState::Released => match capture.selection.state {
                        SelectionState::Clicked(_) => {
                            capture.selection.state = SelectionState::None;
                        }

                        SelectionState::Selecting => {
                            debug!("Requested save");
                            set_foreground_window(capture.formerly_focused_window.0);
                            application.window.set_visible(false);
                            if application.capture_saver.save(capture).is_err() {
                                event_loop.exit();
                                warn!("Exiting: CaptureSaver::save returned Err");
                            }
                            application.renderer.set_hdr_capture(None);
                            if application.renderer.render().is_err() {
                                event_loop.exit();
                                warn!("Exiting: Renderer::render returned Err");
                            }

                            return;
                        }

                        SelectionState::None => {}
                    },
                }

                application.capture = Some(capture);
            }

            WindowEvent::RedrawRequested => {
                // Request render
                if application.renderer.render().is_err() {
                    event_loop.exit();
                    warn!("Exiting: Renderer::render returned Err");
                }
            }

            _ => {}
        }
    }
}

fn is_save_event(event: &KeyEvent) -> bool {
    let keycode = match event.physical_key {
        PhysicalKey::Code(code) => code,
        PhysicalKey::Unidentified(_) => return false,
    };

    event.state == ElementState::Released && keycode == KeyCode::Enter
}

fn is_close_event(event: &KeyEvent) -> bool {
    let keycode = match event.physical_key {
        PhysicalKey::Code(code) => code,
        PhysicalKey::Unidentified(_) => return false,
    };

    event.state == ElementState::Released && keycode == KeyCode::Escape
}
