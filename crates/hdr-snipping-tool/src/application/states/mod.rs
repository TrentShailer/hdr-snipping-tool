use inactive::InactiveApplication;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

use crate::application_event_loop::Event;

use super::events::ApplicationEvent;

mod active;
mod exited;
mod inactive;
mod loading;

pub trait ApplicationState {
    fn handle_event(self: Box<Self>, event: ApplicationEvent) -> Box<dyn ApplicationState>;
}

pub fn initialise_state(
    event_loop: &ActiveEventLoop,
    proxy: EventLoopProxy<Event>,
) -> Box<dyn ApplicationState> {
    Box::new(InactiveApplication::new(event_loop, proxy))
}
