mod create_tray_icon;
mod create_window;

use std::sync::Arc;

use hdr_capture::{
    maximum,
    tonemap::{self, Tonemap},
    Maximum,
};
use thiserror::Error;
use tracing::instrument;
use tray_icon::TrayIcon;
use vulkan_instance::VulkanInstance;
use vulkan_renderer::Renderer;

use windows_capture_provider::{CaptureItemCache, DirectXDevices};
use winit::{event_loop::ActiveEventLoop, window::Window};

use create_tray_icon::create_tray_icon;
use create_window::create_window;

use crate::{
    settings::Settings,
    validation_enabled,
    windows_helpers::foreground_window::{get_foreground_window, set_foreground_window},
};

pub struct ActiveApp {
    pub window: Window,
    #[allow(unused)]
    pub tray_icon: TrayIcon,

    pub vk: Arc<VulkanInstance>,
    pub renderer: Renderer,
    pub maximum: Maximum,
    pub tonemap: Tonemap,

    pub dx: Arc<DirectXDevices>,
    pub capture_item_cache: CaptureItemCache,

    pub settings: Settings,
}

impl ActiveApp {
    #[instrument("ActiveApp::new", skip_all, err)]
    pub fn new(event_loop: &ActiveEventLoop, settings: Settings) -> Result<Self, Error> {
        let focused = get_foreground_window();
        let window = create_window(event_loop)?;
        set_foreground_window(focused);

        let tray_icon = create_tray_icon()?;
        tray_icon.set_visible(true)?;

        let vk = Arc::new(VulkanInstance::new(&window, validation_enabled())?);
        let renderer = Renderer::new(vk.clone(), window.inner_size().into())?;
        let maximum = Maximum::new(vk.clone())?;
        let tonemap = Tonemap::new(vk.clone())?;

        let dx = Arc::new(DirectXDevices::new()?);
        let capture_item_cache = CaptureItemCache::new(&dx)?;

        Ok(ActiveApp {
            window,
            tray_icon,

            vk,
            renderer,
            maximum,
            tonemap,

            dx,
            capture_item_cache,

            settings,
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
    VulkanInstance(#[from] vulkan_instance::CreateError),

    #[error("Failed to create renderer:\n{0}")]
    Renderer(#[from] vulkan_renderer::Error),

    #[error("Failed to create DirectX Devices:\n{0}")]
    DxDevices(#[from] windows_capture_provider::directx_devices::Error),

    #[error("Failed to create Display Cache:\n{0}")]
    DisplayCache(#[from] windows_capture_provider::capture_item_cache::Error),

    #[error("Failed to create maximum finder:\n{0}")]
    Maximum(#[from] maximum::Error),

    #[error("Failed to create tonemapper:\n{0}")]
    Tonemap(#[from] tonemap::Error),
}
