use std::{sync::Arc, time::Instant};

use vulkan_instance::VulkanInstance;
use vulkan_renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub struct ActiveApp {
    pub window_id: WindowId,
    pub window: Arc<Window>,
    pub vulkan: VulkanInstance,
    pub renderer: Renderer,
}

pub struct App {
    pub app: Option<ActiveApp>,
}

impl App {
    pub fn new() -> Self {
        Self { app: None }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("HDR Snipping Tool")
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            .with_active(false)
            .with_visible(false);

        let window = Arc::from(event_loop.create_window(window_attributes).unwrap());
        let window_id = window.id();

        let vulkan = VulkanInstance::new(Arc::clone(&window), event_loop).unwrap();

        let s = Instant::now();
        let renderer = Renderer::new(&vulkan, window.clone()).unwrap();
        let e = Instant::now();
        log::info!("{}ms", e.duration_since(s).as_millis());

        let active_app = ActiveApp {
            vulkan,
            window,
            window_id,
            renderer,
        };

        self.app = Some(active_app);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        // self.handle_user_event(event_loop, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // self.handle_about_to_wait(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // self.handle_window_event(event_loop, window_id, event);
    }
}
