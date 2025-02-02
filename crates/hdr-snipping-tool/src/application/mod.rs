pub use winit_applicaiton::{WindowMessage, WinitApp};

use std::sync::Arc;

use capture::Capture;
use capture_saver::CaptureSaver;
use capture_taker::CaptureTaker;
use renderer::Renderer;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};
use vulkan::Vulkan;
use winit::{
    dpi::PhysicalSize, event_loop::ActiveEventLoop, platform::windows::IconExtWindows,
    raw_window_handle::HasDisplayHandle, window::Window,
};

use crate::{
    config::Config,
    should_debug,
    utilities::{
        failure::{report_and_panic, Failure},
        windows_helpers::{get_foreground_window, set_foreground_window},
    },
    VERSION,
};

mod capture;
mod capture_saver;
mod capture_taker;
mod renderer;
mod winit_applicaiton;

pub const TRAY_SCREENSHOT_ID: &str = "open_screenshot_dir";
pub const TRAY_CONFIG_ID: &str = "open_config_dir";
pub const TRAY_QUIT_ID: &str = "quit";

#[allow(unused)]
pub struct Application {
    window: Window,
    tray_icon: TrayIcon,

    vulkan: Arc<Vulkan>,
    renderer: Renderer,
    capture_taker: CaptureTaker,
    capture_saver: CaptureSaver,

    capture: Option<Capture>,
}

impl Application {
    pub fn new(event_loop: &ActiveEventLoop, config: Config) -> Self {
        // Create the window
        let window = {
            let focused_window = get_foreground_window();

            let window_icon =
                winit::window::Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))
                    .report_and_panic("Could not create the window");

            let window_attributes = Window::default_attributes()
                .with_title("HDR Snipping Tool")
                .with_window_icon(Some(window_icon))
                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
                .with_active(false)
                .with_visible(false);

            let window = event_loop
                .create_window(window_attributes)
                .report_and_panic("Could not create the window");

            set_foreground_window(focused_window);

            window
        };

        // Create tray icon
        let tray_icon = {
            let screenshot_item =
                MenuItem::with_id(TRAY_SCREENSHOT_ID, "Open Screenshot Directory", true, None);
            let config_item =
                MenuItem::with_id(TRAY_CONFIG_ID, "Open Config Directory", true, None);
            let quit_item = MenuItem::with_id(TRAY_QUIT_ID, "Quit HDR Snipping Tool", true, None);

            let tray_menu = Menu::with_items(&[&screenshot_item, &config_item, &quit_item])
                .report_and_panic("Could not create tray icon");
            let icon = tray_icon::Icon::from_resource(1, Some((24, 24)))
                .report_and_panic("Could not create tray icon");
            let tooltip = format!("HDR Snipping Tool v{}", VERSION);

            TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip(tooltip)
                .with_icon(icon)
                .build()
                .report_and_panic("Could not create tray icon")
        };

        // Initialise Vulkan
        let vulkan = {
            let result = unsafe {
                Vulkan::new(
                    should_debug(),
                    Some(window.display_handle().unwrap().as_raw()),
                )
            };

            match result {
                Ok(vulkan) => Arc::new(vulkan),
                Err(error) => match error {
                    vulkan::VulkanCreationError::VkError(vk_error) => {
                        report_and_panic(vk_error, "Could not initialise Vulkan")
                    }

                    vulkan::VulkanCreationError::UnsupportedInstance => {
                        report_and_panic(
                            error,
                            "Your GPU does not meet the requirements to run this application",
                        );
                    }

                    vulkan::VulkanCreationError::UnsupportedDevice => {
                        report_and_panic(
                            error,
                            "Your GPU does not meet the requirements to run this application",
                        );
                    }
                },
            }
        };

        let renderer = Renderer::new(vulkan.clone(), &window);
        let capture_taker = CaptureTaker::new(vulkan.clone());
        let capture_saver = CaptureSaver::new(vulkan.clone());

        Self {
            window,
            tray_icon,

            vulkan,

            renderer,
            capture_taker,
            capture_saver,

            capture: None,
        }
    }
}
