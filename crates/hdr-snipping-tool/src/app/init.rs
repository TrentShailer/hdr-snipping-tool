use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkan_renderer::renderer::{self, Renderer};
use winit::{
    dpi::PhysicalSize,
    error::OsError,
    event_loop::ActiveEventLoop,
    platform::windows::IconExtWindows,
    window::{BadIcon, Icon, Window},
};

use super::{
    tray_icon::{self, init_tray_icon},
    ActiveApp, App,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create icon:\n{0}")]
    Icon(#[from] BadIcon),

    #[error("Failed to create window:\n{0}")]
    CreateWindow(#[from] OsError),

    #[error("Failed to create tray icon:\n{0}")]
    TrayIcon(#[from] tray_icon::Error),

    #[error("Failed to make tray icon visible:\n{0}")]
    TrayIconVisible(#[from] ::tray_icon::Error),

    #[error("Failed to create vulkan instance:\n{0}")]
    VulkanInstance(#[from] vulkan_instance::vulkan_instance::Error),

    #[error("Failed to create renderer:\n{0}")]
    Renderer(#[from] renderer::Error),
}

impl App {
    pub fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Error> {
        let window_icon = Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))?;
        // create window
        let window_attributes = Window::default_attributes()
            .with_title("HDR Snipping Tool")
            .with_window_icon(Some(window_icon))
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            .with_active(false)
            .with_visible(false);

        let window = Arc::from(event_loop.create_window(window_attributes)?);
        let window_id = window.id();

        let tray_icon = init_tray_icon()?;
        tray_icon.set_visible(true)?;

        let vulkan_instance = VulkanInstance::new(Arc::clone(&window), event_loop)?;

        let renderer = Renderer::new(&vulkan_instance, window.clone())?;

        let active_app = ActiveApp {
            tray_icon,
            vulkan_instance,
            window,
            window_id,
            renderer,
        };

        self.app = Some(active_app);

        Ok(())
    }
}
