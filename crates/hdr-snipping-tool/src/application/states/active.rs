use tracing::debug;
use windows::Win32::Foundation::HWND;

use crate::{
    application::{
        KeyboardEvent, MouseEvent, WindowEvent, capture_resources::CaptureResources,
        core_resources::CoreResources,
    },
    capture_saver::CaptureSaver,
    selection::{Selection, SelectionEvent, SelectionState},
};

use super::{
    ApplicationEvent, ApplicationState, exited::ExitedApplication, inactive::InactiveApplication,
    loading::LoadingApplication,
};

pub struct ActiveApplication {
    pub core: CoreResources,
    pub capture: CaptureResources,
    pub previous_focused_window: HWND,
    pub selection: Option<Box<dyn SelectionState>>,
}

impl ActiveApplication {
    fn handle_selection_update(mut self: Box<Self>) -> Box<dyn ApplicationState> {
        let Some(state) = self.selection.as_ref() else {
            return self;
        };

        let Some(selection) = state.selection() else {
            return self;
        };

        self.capture.selection = selection;
        self.core.renderer.set_selection(selection);
        self.core.window.request_redraw();

        if state.is_submitted() {
            self.save()
        } else {
            self
        }
    }

    fn save(self: Box<Self>) -> Box<dyn ApplicationState> {
        debug!("Saving");

        self.core.capture_saver.save_capture(
            self.capture.hdr_capture,
            self.capture.whitepoint.value(),
            self.capture.selection,
        );

        Box::new(InactiveApplication::from(*self))
    }

    fn cancel(self: Box<Self>) -> Box<dyn ApplicationState> {
        debug!("Cancelling");
        Box::new(InactiveApplication::from(*self))
    }
}

impl ApplicationState for ActiveApplication {
    fn handle_event(mut self: Box<Self>, event: ApplicationEvent) -> Box<dyn ApplicationState> {
        match event {
            ApplicationEvent::MouseEvent(mouse_event) => {
                if let MouseEvent::Moved(position) = mouse_event {
                    self.core.renderer.set_mouse_position(position);
                    self.core.window.request_redraw();
                }

                let Some(selection) = self.selection.take() else {
                    if let MouseEvent::Clicked(position) = mouse_event {
                        self.selection = Some(Selection::mouse_clicked(position));
                    }
                    return self;
                };

                self.selection = match mouse_event {
                    MouseEvent::Moved(physical_position) => {
                        selection.handle_event(SelectionEvent::MouseMoved(physical_position))
                    }
                    MouseEvent::Released => selection.handle_event(SelectionEvent::MouseReleased),
                    _ => Some(selection),
                };

                self.handle_selection_update()
            }

            ApplicationEvent::KeyboardEvent(keyboard_event) => match keyboard_event {
                KeyboardEvent::EscapePressed => self.cancel(),
                KeyboardEvent::EnterPressed => self.save(),
            },

            ApplicationEvent::WindowEvent(window_event) => match window_event {
                WindowEvent::RedrawRequested => {
                    self.core.renderer.render();
                    self
                }
                WindowEvent::Resized => {
                    self.core.renderer.resize();
                    self.core.window.request_redraw();
                    self
                }
            },

            ApplicationEvent::Shutdown => Box::new(ExitedApplication::from(*self)),

            _ => self,
        }
    }
}

impl From<LoadingApplication> for ActiveApplication {
    fn from(application: LoadingApplication) -> Self {
        debug!("[TRANSITION] Loading -> Active");

        let core = application.core;
        let capture = CaptureResources {
            monitor: application
                .monitor
                .expect("Transition to active requires monitor to be Some"),

            capture: application
                .capture
                .expect("Transition to active requires capture to be Some"),

            hdr_capture: application
                .hdr_capture
                .expect("Transition to active requires hdr_capture to be Some"),

            whitepoint: application
                .whitepoint
                .expect("Transition to active requires whitepoint to be Some"),

            selection: application
                .selection
                .expect("Transition to active requires selection to be Some"),
        };

        Self {
            core,
            capture,
            previous_focused_window: application.previous_focused_window,
            selection: None,
        }
    }
}
