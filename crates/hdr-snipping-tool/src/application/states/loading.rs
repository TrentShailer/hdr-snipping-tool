use tracing::debug;
use vulkan::HdrImage;
use windows::Win32::Foundation::HWND;
use windows_capture_provider::{Monitor, WindowsCapture};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::{
    application::{LoadingEvent, WindowEvent, core_resources::CoreResources},
    capture_taker::{CaptureTaker, Whitepoint},
    selection::Selection,
    utilities::{failure::Ignore, windows_helpers::get_foreground_window},
};

use super::{
    ApplicationEvent, ApplicationState, active::ActiveApplication, exited::ExitedApplication,
    inactive::InactiveApplication,
};

pub struct LoadingApplication {
    pub core: CoreResources,
    pub previous_focused_window: HWND,
    pub monitor: Option<Monitor>,
    pub selection: Option<Selection>,
    pub capture: Option<WindowsCapture>,
    pub hdr_capture: Option<HdrImage>,
    pub whitepoint: Option<Whitepoint>,
    pub is_visible: bool,
}

impl LoadingApplication {
    fn update_window(&mut self) {
        let mut should_redraw = false;

        if let Some(monitor) = self.monitor.as_ref() {
            let window_size = monitor.size();
            let window_size = PhysicalSize::new(window_size[0], window_size[1]);

            if self.core.window.inner_size() != window_size {
                self.core.window.request_inner_size(window_size).ignore();
            }

            let window_position = PhysicalPosition::new(
                monitor.desktop_coordinates.left,
                monitor.desktop_coordinates.top,
            );
            if self.core.window.outer_position().unwrap() != window_position {
                self.core.window.set_outer_position(window_position);
            }
            should_redraw = true;
        }

        if self.hdr_capture.is_some() && !self.is_visible {
            self.is_visible = true;
            self.core.window.set_visible(true);
            self.core.window.focus_window();
            should_redraw = true;
        }

        if self.is_visible {
            should_redraw = true;
        }

        if should_redraw {
            self.core.window.request_redraw();
            self.core.renderer.render();
        }
    }

    fn is_finished_loading(&self) -> bool {
        self.monitor.is_some()
            && self.capture.is_some()
            && self.hdr_capture.is_some()
            && self.whitepoint.is_some()
            && self.selection.is_some()
    }

    fn transition_if_finished(mut self: Box<Self>) -> Box<dyn ApplicationState> {
        if self.is_finished_loading() {
            self.update_window();
            Box::new(ActiveApplication::from(*self))
        } else {
            self
        }
    }
}

impl ApplicationState for LoadingApplication {
    fn handle_event(mut self: Box<Self>, event: ApplicationEvent) -> Box<dyn ApplicationState> {
        match event {
            ApplicationEvent::LoadingEvent(event) => match event {
                LoadingEvent::FoundMonitor(monitor) => {
                    self.monitor = Some(monitor);
                    let selection = {
                        let size = monitor.size();
                        Selection {
                            start: PhysicalPosition::default(),
                            end: PhysicalPosition::new(size[0] as f32, size[1] as f32),
                        }
                    };
                    self.selection = Some(selection);

                    self.core.renderer.set_selection(selection);

                    self.update_window();
                    self.transition_if_finished()
                }

                LoadingEvent::GotCapture(windows_capture) => {
                    self.capture = Some(windows_capture);

                    self.update_window();
                    self.transition_if_finished()
                }

                LoadingEvent::ImportedCapture(hdr_image) => {
                    self.hdr_capture = Some(hdr_image);
                    self.core.renderer.set_hdr_capture(self.hdr_capture);
                    self.update_window();
                    self.transition_if_finished()
                }

                LoadingEvent::SelectedWhitepoint(whitepoint) => {
                    self.whitepoint = Some(whitepoint);
                    self.core.renderer.set_whitepoint(whitepoint.value());
                    self.core.renderer.set_max_brightness(whitepoint.value());
                    self.update_window();
                    self.transition_if_finished()
                }

                LoadingEvent::Error => Box::new(InactiveApplication::from(*self)),
            },

            ApplicationEvent::WindowEvent(window_event) => match window_event {
                WindowEvent::RedrawRequested => {
                    self.core.renderer.render();
                    self
                }
                WindowEvent::Resized => {
                    self.core.renderer.resize();
                    self
                }
            },

            ApplicationEvent::Shutdown => Box::new(ExitedApplication::from(*self)),

            _ => self,
        }
    }
}

impl From<InactiveApplication> for LoadingApplication {
    fn from(application: InactiveApplication) -> Self {
        debug!("[TRANSITION] Inactive -> Loading");

        let mut application = Self {
            core: application.core,
            previous_focused_window: get_foreground_window(),
            monitor: None,
            selection: None,
            capture: None,
            hdr_capture: None,
            whitepoint: None,
            is_visible: false,
        };

        application
            .core
            .capture_taker
            .take_capture(application.core.proxy.clone());

        application
    }
}
