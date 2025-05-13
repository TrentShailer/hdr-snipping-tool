use ::tray_icon::menu::MenuEvent;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::WindowId,
};

use crate::{
    application::{
        self, ApplicationEvent, ApplicationState, KeyboardEvent, MouseEvent, initialise_state,
    },
    utilities::failure::Ignore,
};

pub use tray_icon::TrayIcon;

mod tray_icon;

pub enum Event {
    ApplicationEvent(ApplicationEvent),
    TrayEvent(MenuEvent),
}

pub struct ApplicationEventLoop {
    proxy: EventLoopProxy<Event>,
    mouse_position: PhysicalPosition<f32>,
    state: Option<Box<dyn ApplicationState>>,
}

impl ApplicationEventLoop {
    pub fn new(proxy: EventLoopProxy<Event>) -> Self {
        Self {
            proxy,
            mouse_position: PhysicalPosition::default(),
            state: None,
        }
    }
}

impl ApplicationHandler<Event> for ApplicationEventLoop {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(initialise_state(event_loop, self.proxy.clone()));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.state.is_none() {
            return;
        }

        if event == WindowEvent::Destroyed {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(_) => self
                .proxy
                .send_event(application::WindowEvent::Resized.into())
                .ignore(),

            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let Key::Named(named_key) = event.logical_key {
                    match named_key {
                        NamedKey::Enter => self
                            .proxy
                            .send_event(KeyboardEvent::EnterPressed.into())
                            .ignore(),

                        NamedKey::Escape => self
                            .proxy
                            .send_event(KeyboardEvent::EscapePressed.into())
                            .ignore(),

                        _ => {}
                    }
                }
            }

            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.cast();
                self.proxy
                    .send_event(MouseEvent::Moved(self.mouse_position).into())
                    .ignore();
            }

            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => self
                            .proxy
                            .send_event(MouseEvent::Clicked(self.mouse_position).into())
                            .ignore(),

                        ElementState::Released => {
                            self.proxy.send_event(MouseEvent::Released.into()).ignore()
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => self
                .proxy
                .send_event(application::WindowEvent::RedrawRequested.into())
                .ignore(),

            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        match event {
            Event::ApplicationEvent(application_event) => {
                if let Some(state) = self.state.take() {
                    self.state = Some(state.handle_event(application_event));
                }
            }

            Event::TrayEvent(menu_event) => TrayIcon::handle_event(event_loop, menu_event),
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        if let Some(state) = self.state.take() {
            let _state = state.handle_event(ApplicationEvent::Shutdown);
        }
    }
}
