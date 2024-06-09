use std::{sync::Arc, time::Instant};

use half::f16;
use hdr_capture::CaptureProvider;
use vulkan_instance::texture::Texture;
use vulkan_tonemapper::Tonemapper;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};

use crate::{message_box::display_message, selection::Selection, App};

use super::ActiveCapture;

// TODO make inner function for error handling

impl App {
    pub(super) fn handle_user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        // If window is not visible, take and present capture
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return,
        };

        if app.window.is_visible().unwrap_or(true) {
            return;
        }

        let start = Instant::now();

        let s = Instant::now();
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
        let e = Instant::now();
        log::info!("Got capture in {}ms", e.duration_since(s).as_millis());

        let _physical_size = app.window.request_inner_size(capture_info.size);
        app.window.set_outer_position(display_info.position);

        let s = Instant::now();
        let texture = match Texture::new(&app.vulkan_instance, capture_info.size) {
            Ok(v) => Arc::new(v),
            Err(e) => {
                log::error!("{e}");
                display_message(
                    "We encountered an error while creating the output texure.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
                std::process::exit(-1);
            }
        };
        let e = Instant::now();
        log::info!("Created texture in {}ms", e.duration_since(s).as_millis());

        let s = Instant::now();
        let mut tonemapper = match Tonemapper::new(
            &app.vulkan_instance,
            texture.clone(),
            &raw_capture,
            capture_info.size,
            f16::from_f32(1.0),
            f16::from_f32(self.settings.default_gamma),
        ) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                display_message(
                    "We encountered an error while creating the tonemapper.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
                std::process::exit(-1);
            }
        };
        let e = Instant::now();
        log::info!(
            "Created tonemapper in {}ms",
            e.duration_since(s).as_millis()
        );

        let s = Instant::now();
        if let Err(e) = tonemapper.tonemap(&app.vulkan_instance) {
            log::error!("{e}");
            display_message(
                    "We encountered an error while tonemapping the capture.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
            std::process::exit(-1);
        }
        let e = Instant::now();
        log::info!("Tonemapped in {}ms", e.duration_since(s).as_millis());

        let end = Instant::now();
        log::info!("Total {}ms", end.duration_since(start).as_millis());

        if let Err(e) = app
            .renderer
            .load_texture(&app.vulkan_instance, texture.clone())
        {
            log::error!("{e}");
            display_message(
                    "We encountered an error while loading the capture into the renderer.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
            std::process::exit(-1);
        }

        let capture = ActiveCapture {
            texture,
            tonemapper,
        };

        self.capture = Some(capture);

        self.selection = Selection::new(
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(display_info.size.width, display_info.size.height),
        );

        app.window.set_visible(true);
        app.window.focus_window();
    }
}
