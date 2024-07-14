use thiserror::Error;

use crate::active_capture;

use super::{adjust_tonemap_settings, clear_capture};

pub mod keyboard_input;
pub mod mouse_input;
pub mod mouse_wheel;
pub mod redraw;
pub mod simple_events;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to render:\n{0}")]
    Render(#[from] vulkan_renderer::renderer::render::Error),

    #[error("Failed to clear the capture:\n{0}")]
    ClearCapture(#[from] clear_capture::Error),

    #[error("Failed to adjust tonemapping settings:\n{0}")]
    AdjustTonemapSettings(#[from] adjust_tonemap_settings::Error),

    #[error("Failed to save capture:\n{0}")]
    SaveCapture(#[from] active_capture::save::Error),
}
