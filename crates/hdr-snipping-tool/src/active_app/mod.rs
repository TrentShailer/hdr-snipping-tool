mod create_tray_icon;
mod create_window;
pub mod handle_tray_icon;
pub mod take_capture;

use std::sync::Arc;

use scrgb_tonemapper::maximum::{self, Maximum};
use thiserror::Error;
use tracing::info_span;
use tray_icon::TrayIcon;
use vulkan_instance::VulkanInstance;
use vulkan_renderer::renderer::Renderer;

use windows_capture_provider::{DirectXDevices, DisplayCache};
use winit::{event_loop::ActiveEventLoop, window::Window};

use create_tray_icon::create_tray_icon;
use create_window::create_window;

use crate::{
    is_vk_debug,
    windows_helpers::foreground_window::{get_foreground_window, set_foreground_window},
};

pub struct ActiveApp {
    pub window: Arc<Window>,
    pub _tray_icon: TrayIcon,
    pub vk: VulkanInstance,
    pub renderer: Renderer,
    pub dx: DirectXDevices,
    pub display_cache: DisplayCache,
    pub maximum: Maximum,
}

impl ActiveApp {
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Self, Error> {
        let _span = info_span!("ActiveApp::new").entered();

        let focused = get_foreground_window();
        let window = create_window(event_loop)?;
        set_foreground_window(focused);

        let tray_icon = create_tray_icon()?;
        tray_icon.set_visible(true)?;

        let vk = VulkanInstance::new(window.clone(), is_vk_debug())?;
        let renderer = Renderer::new(&vk, window.clone())?;

        let dx = DirectXDevices::new()?;
        let display_cache = DisplayCache::new(&dx)?;

        let maximum = Maximum::new(&vk)?;

        Ok(ActiveApp {
            window,
            vk,
            renderer,
            dx,
            display_cache,
            maximum,
            _tray_icon: tray_icon,
        })
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

    #[error("Failed to create DirectX Devices:\n{0}")]
    DxDevices(#[from] windows_capture_provider::directx_devices::Error),

    #[error("Failed to create Display Cache:\n{0}")]
    DisplayCache(#[from] windows_capture_provider::display_cache::Error),

    #[error("Failed to create maximum finder:\n{0}")]
    Maximum(#[from] maximum::Error),
}
