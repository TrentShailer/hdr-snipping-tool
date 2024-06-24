mod about_to_wait;
mod init;
mod save;
mod tray_icon;
mod user_event;
mod window_event;

use std::{sync::Arc, time::Instant};

use ::tray_icon::TrayIcon;
use vulkan_instance::{texture::Texture, VulkanInstance};
use vulkan_renderer::renderer::Renderer;
use vulkan_tonemapper::Tonemapper;
use windows_capture_provider::WindowsCaptureProvider;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{selection::Selection, settings::Settings};

pub struct ActiveApp {
    pub window_id: WindowId,
    pub window: Arc<Window>,
    pub tray_icon: TrayIcon,
    pub vulkan_instance: VulkanInstance,
    pub renderer: Renderer,
}

pub struct ActiveCapture {
    pub tonemapper: Tonemapper,
    pub texture: Arc<Texture>,
}

pub struct App {
    pub app: Option<ActiveApp>,
    pub capture: Option<ActiveCapture>,
    pub capture_provider: WindowsCaptureProvider,
    pub settings: Settings,
    pub mouse_position: PhysicalPosition<i32>,
    pub selection: Selection,
    pub last_frame: Instant,
}

impl App {
    pub fn new(capture_provider: WindowsCaptureProvider, settings: Settings) -> Self {
        Self {
            capture_provider,
            settings,
            app: None,
            capture: None,
            mouse_position: PhysicalPosition::default(),
            selection: Selection::default(),
            last_frame: Instant::now(),
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.init(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        self.handle_user_event(event_loop, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_about_to_wait(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }
}
