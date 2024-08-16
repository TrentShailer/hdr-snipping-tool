use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::windows::EventLoopBuilderExtWindows,
    window::{Window, WindowId},
};

pub fn create_event_loop() -> EventLoop<()> {
    let event_loop: EventLoop<()> = EventLoop::with_user_event()
        .with_any_thread(true)
        .build()
        .unwrap();
    event_loop
}

pub fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let window_attributes = Window::default_attributes()
        .with_title("TEST WINDOW")
        .with_active(false)
        .with_visible(false);

    let window = event_loop.create_window(window_attributes).unwrap();
    Arc::from(window)
}
pub struct App<F: Fn(&ActiveEventLoop, Arc<Window>), W: Fn(&ActiveEventLoop, WindowId, WindowEvent)>
{
    pub resumed_callback: F,
    pub window_event_callback: W,
    pub window: Option<Arc<Window>>,
}

impl<F: Fn(&ActiveEventLoop, Arc<Window>), W: Fn(&ActiveEventLoop, WindowId, WindowEvent)>
    ApplicationHandler for App<F, W>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = create_window(event_loop);
        self.window = Some(window.clone());
        (self.resumed_callback)(event_loop, window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        (self.window_event_callback)(event_loop, window_id, event)
    }
}
