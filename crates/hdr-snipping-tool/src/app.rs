mod about_to_wait;
mod init;
mod save;
mod tray_icon;
mod user_event;
mod window_event;

use std::{sync::Arc, time::Instant};

use ::tray_icon::TrayIcon;
use vulkan_backend::VulkanInstance;
use windows_capture_provider::WindowsCaptureProvider;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{init::settings::Settings, selection::Selection};

pub struct App {
    pub capture_provider: WindowsCaptureProvider,
    pub settings: Settings,
    pub window_id: Option<WindowId>,
    pub window: Option<Arc<Window>>,
    pub tray_icon: Option<TrayIcon>,
    pub vulkan_instance: Option<VulkanInstance>,
    pub mouse_position: PhysicalPosition<i32>,
    pub selection: Selection,
    pub last_frame: Instant,
}

impl App {
    pub fn new(capture_provider: WindowsCaptureProvider, settings: Settings) -> Self {
        Self {
            capture_provider,
            settings,
            window_id: None,
            window: None,
            tray_icon: None,
            vulkan_instance: None,
            mouse_position: PhysicalPosition::default(),
            selection: Selection::default(),
            last_frame: Instant::now(),
        }
    }

    fn is_visible(window: &Option<Arc<Window>>) -> bool {
        let window = match window.as_ref() {
            Some(v) => v,
            None => return false,
        };

        window.is_visible().unwrap_or(true)
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
