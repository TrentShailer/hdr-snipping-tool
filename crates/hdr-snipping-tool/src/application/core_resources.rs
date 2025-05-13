use std::sync::Arc;

use vulkan::Vulkan;
use winit::{
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    platform::windows::IconExtWindows,
    raw_window_handle::HasDisplayHandle,
    window::Window,
};

use crate::{
    application_event_loop::Event,
    capture_saver::CaptureSaverThread,
    capture_taker::CaptureTakerThread,
    config_dir,
    renderer_thread::RendererThread,
    should_debug,
    utilities::{
        failure::{Failure, report_and_panic},
        windows_helpers::{get_foreground_window, set_foreground_window},
    },
};

pub struct CoreResources {
    pub window: Window,
    pub vulkan: Arc<Vulkan>,
    pub capture_saver: CaptureSaverThread,
    pub capture_taker: CaptureTakerThread,
    pub renderer: RendererThread,
    pub proxy: EventLoopProxy<Event>,
}

impl CoreResources {
    pub fn new(event_loop: &ActiveEventLoop, proxy: EventLoopProxy<Event>) -> Self {
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

        // Initialise Vulkan
        let vulkan = {
            let result = Vulkan::new(
                should_debug(),
                &config_dir(),
                Some(window.display_handle().unwrap().as_raw()),
            );

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

                    error => report_and_panic(error, "Could not initialise Vulkan"),
                },
            }
        };

        let capture_saver = CaptureSaverThread::new(Arc::clone(&vulkan));
        let capture_taker = CaptureTakerThread::new(Arc::clone(&vulkan));
        let renderer = RendererThread::new(Arc::clone(&vulkan), &window);

        Self {
            window,
            vulkan,
            capture_saver,
            capture_taker,
            renderer,
            proxy,
        }
    }
}
