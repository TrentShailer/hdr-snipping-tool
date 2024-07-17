pub mod create_tray_icon;
pub mod create_window;

use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkan_renderer::renderer::Renderer;
use windows_capture_provider::WindowsCaptureProvider;
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop, keyboard::ModifiersState};

use crate::active_app::ActiveApp;

use super::WinitApp;

impl WinitApp {
    pub fn on_resume(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Error> {
        let window = Self::create_window(event_loop)?;

        let tray_icon = Self::create_tray_icon()?;
        tray_icon.set_visible(true)?;

        let vk = Arc::new(VulkanInstance::new(window.clone(), event_loop)?);
        let renderer = Renderer::new(&vk, window.clone())?;

        let capture_provider = WindowsCaptureProvider::new()?;

        self.app = Some(ActiveApp {
            capture_provider,
            renderer,
            vk,
            window,
            active_capture: None,
            keyboard_modifiers: ModifiersState::empty(),
            mouse_position: PhysicalPosition::default(),
            scroll: 0.0,
            _tray_icon: tray_icon,
        });

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create window:\n{0}")]
    Window(#[from] create_window::Error),

    #[error("Failed to create tray icon:\n{0}")]
    TrayIcon(#[from] create_tray_icon::Error),

    #[error("Failed to make tray icon visible:\n{0}")]
    TrayIconVisible(#[from] tray_icon::Error),

    #[error("Failed to create vulkan instance:\n{0}")]
    VulkanInstance(#[from] vulkan_instance::vulkan_instance::Error),

    #[error("Failed to create renderer:\n{0}")]
    Renderer(#[from] vulkan_renderer::renderer::Error),

    #[error("Failed to capture provider:\n{0}")]
    CaptureProvider(#[from] windows_capture_provider::Error),
}
