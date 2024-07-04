use std::sync::Arc;

use half::f16;
use hdr_capture::CaptureProvider;
use thiserror::Error;
use vulkan_instance::texture::Texture;
use vulkan_renderer::{capture, text};
use vulkan_tonemapper::{tonemapper, Tonemapper};
use winit::dpi::PhysicalPosition;

use crate::{selection::Selection, App};

use super::ActiveCapture;

impl App {
    pub fn take_capture(&mut self) -> Result<(), Error> {
        // If window is not visible, take and present capture
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        if app.window.is_visible().unwrap_or(true) {
            return Ok(());
        }

        let (raw_capture, display_info, capture_info) = self.capture_provider.get_capture()?;

        let _physical_size = app.window.request_inner_size(capture_info.size);
        app.window.set_outer_position(display_info.position);

        let texture = Arc::new(Texture::new(&app.vulkan_instance, capture_info.size)?);

        let mut tonemapper = Tonemapper::new(
            &app.vulkan_instance,
            texture.clone(),
            &raw_capture,
            capture_info.size,
            f16::from_f32(1.0),
            f16::from_f32(self.settings.default_gamma),
        )?;

        app.renderer.parameters.set_parameters(
            &app.vulkan_instance,
            &mut app.renderer.glyph_cache,
            tonemapper.config.alpha,
            tonemapper.config.gamma,
            tonemapper.config.maximum,
        )?;

        tonemapper.tonemap(&app.vulkan_instance)?;

        app.renderer
            .capture
            .load_capture(&app.vulkan_instance, texture.clone())?;

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

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get capture from provider:\n{0}")]
    CaptureProvider(#[from] windows_capture_provider::get_capture::Error),

    #[error("Failed to create texture:\n{0}")]
    Texture(#[from] vulkan_instance::texture::Error),

    #[error("Failed to create tonemapper:\n{0}")]
    Tonemapper(#[from] tonemapper::Error),

    #[error("Failed to update renderer:\n{0}")]
    UpdateRenderer(#[from] text::set_text::Error),

    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] tonemapper::tonemap::Error),

    #[error("Failed to load capture into renderer:\n{0}")]
    LoadCapture(#[from] capture::load::Error),
}
