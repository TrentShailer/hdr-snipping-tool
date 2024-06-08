use half::f16;
use hdr_capture::CaptureProvider;
use vulkan_backend::texture::Texture;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};

use crate::{message_box::display_message, selection::Selection, App};

// TODO make inner function for error handling

impl App {
    pub(super) fn handle_user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        // If window is not visible, take and present capture
        let window = match self.window.as_ref() {
            Some(v) => v,
            None => return,
        };

        let backend = match self.backend.as_mut() {
            Some(v) => v,
            None => return,
        };

        let vulkan = match self.vulkan.as_ref() {
            Some(v) => v,
            None => return,
        };

        if Self::is_visible(&self.window) {
            return;
        }

        let (raw_capture, display_info, capture_info) = match self.capture_provider.get_capture() {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                display_message(
                    "We encountered an error while getting a capture from the provider.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
                std::process::exit(-1);
            }
        };

        let _physical_size = window.request_inner_size(capture_info.size);
        window.set_outer_position(display_info.position);

        if let Err(e) = backend.tonemapper.load_capture(
            &vulkan,
            &raw_capture,
            f16::from_f32(1.0), // 1.0 alpha will always tonemap so there is no clipping
            f16::from_f32(self.settings.default_gamma),
            capture_info.size,
        ) {
            log::error!("{e}");
            display_message(
                "We encountered an error while loading your capture into Vulkan.\nMore details are in the logs.",
                MB_ICONERROR,
            );
            std::process::exit(-1);
        };

        let texture = match Texture::new(&vulkan, capture_info.size) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                display_message(
                "We encountered an error while creating the texture for your capture.\nMore details are in the logs.",
                MB_ICONERROR,
            );
                std::process::exit(-1);
            }
        };

        if let Err(e) = backend.tonemapper.tonemap(&vulkan, texture.image.clone()) {
            log::error!("{e}");
            display_message(
                "We encountered an error while tonemapping the capture.\nMore details are in the logs.",
                MB_ICONERROR,
            );
            std::process::exit(-1);
        }

        if let Err(e) = backend
            .renderer
            .renderpass_capture
            .load_image(&vulkan, texture)
        {
            log::error!("{e}");
            display_message(
                "We encountered an error while loading the capture to the renderer.\nMore details are in the logs.",
                MB_ICONERROR,
            );
            std::process::exit(-1);
        };

        self.selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(display_info.size.width, display_info.size.height),
        );

        window.set_visible(true);
        window.focus_window();
    }
}
