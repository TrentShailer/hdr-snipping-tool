use std::process::Command;

use tracing::{debug, info, warn};
use tray_icon::menu::MenuEvent;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::EventLoopProxy,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    config::Config,
    config_dir, screenshot_dir,
    selection::SelectionState,
    utilities::{failure::Failure, windows_helpers::set_foreground_window},
};

use super::{
    capture::Capture, capture_taker::CaptureProgress, Application, TRAY_CONFIG_ID, TRAY_QUIT_ID,
    TRAY_SCREENSHOT_ID,
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
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let application = Application::new(event_loop, self.config);
        self.application = Some(application);
    }

    fn user_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: WindowMessage,
    ) {
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
                        let capture = Capture::new(application.vulkan.clone(), monitor);

                        // Update window
                        {
                            application.window.set_outer_position(PhysicalPosition::new(
                                monitor.desktop_coordinates.left,
                                monitor.desktop_coordinates.top,
                            ));

                            let size = monitor.size();
                            let _ = application
                                .window
                                .request_inner_size(PhysicalSize::new(size[0], size[1]));
                        }

                        // Update renderer
                        {
                            application.renderer.set_mouse_position(self.mouse_position);
                            application.renderer.set_selection(capture.selection);
                        }

                        // Request redraw
                        if application.renderer.render().is_err() {
                            event_loop.exit();
                            warn!("Exiting: Renderer::render returned Err");
                        }

                        application.capture = Some(capture);
                    }

                    CaptureProgress::CaptureTaken(windows_capture) => {
                        if let Some(capture) = application.capture.as_mut() {
                            capture.windows_capture = Some(windows_capture);

                            application.window.set_visible(true);
                            application.window.focus_window();
                        }
                    }

                    CaptureProgress::Imported(hdr_image) => {
                        if let Some(capture) = application.capture.as_mut() {
                            capture.hdr_capture = Some(hdr_image);
                            application.renderer.set_hdr_capture(capture.hdr_capture);
                        }
                    }

                    CaptureProgress::FoundWhitepoint(whitepoint) => {
                        if let Some(capture) = application.capture.as_mut() {
                            capture.whitepoint = whitepoint;
                            application.renderer.set_whitepoint(capture.whitepoint);
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

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.application.take();
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }

        let Some(application) = self.application.as_mut() else {
            return;
        };

        // Request render
        if application.renderer.render().is_err() {
            event_loop.exit();
            warn!("Exiting: Renderer::render returned Err");
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
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if event_loop.exiting() {
            return;
        }

        let Some(application) = self.application.as_mut() else {
            return;
        };

        if event == winit::event::WindowEvent::Destroyed && application.window.id() == window_id {
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
                }
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
                    }

                    // Save
                    ElementState::Released => match capture.selection.state {
                        SelectionState::Clicked(_) => {
                            capture.selection.state = SelectionState::None
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
