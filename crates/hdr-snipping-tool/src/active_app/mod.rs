pub mod adjust_tonemap_settings;
pub mod clear_capture;
pub mod take_capture;
pub mod tray_icon;
pub mod window_event;

use std::sync::Arc;

use ::tray_icon::TrayIcon;
use vulkan_instance::VulkanInstance;
use vulkan_renderer::renderer::Renderer;
use windows_capture_provider::WindowsCaptureProvider;
use winit::{dpi::PhysicalPosition, keyboard::ModifiersState, window::Window};

use crate::active_capture::ActiveCapture;

pub struct ActiveApp {
    pub window: Arc<Window>,
    pub _tray_icon: TrayIcon,
    pub renderer: Renderer,
    pub vk: VulkanInstance,
    pub capture_provider: WindowsCaptureProvider,
    pub mouse_position: PhysicalPosition<u32>,
    pub scroll: f32,
    pub keyboard_modifiers: ModifiersState,
    pub active_capture: Option<ActiveCapture>,
}
