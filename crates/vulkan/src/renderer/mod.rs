use alloc::sync::Arc;
use ash_helper::{
    AllocationError, Swapchain, SwapchainPreferences, SwapchainRetirement, VkError, VulkanContext,
};
use buffer::RenderBuffer;
use context::Surface;
use parking_lot::Mutex;
use pipelines::{CapturePipeline, LinePipeline, SelectionPipeline};
use thiserror::Error;
use tracing::error;

use crate::{HdrImage, Vulkan};

mod buffer;
mod context;
mod new;
mod pipelines;
mod render;

/// The dynamic state for the renderer, updates are provided externally.
#[derive(Default, Clone, Copy)]
pub struct State {
    /// The whitepoint for previewing the tonemap.
    pub whitepoint: f32,

    /// The monitors max brightness.
    pub max_brightness: f32,

    /// The HDR Capture.
    pub capture: Option<HdrImage>,

    /// The area (start, end), that the user has currently selected, relative to the top-left
    /// corner of the window.
    pub selection: [[f32; 2]; 2],

    /// The position of the user's mouse relative to the top-left corner of the window.
    pub mouse_position: [f32; 2],
}

/// The renderer for HDR Snipping Tool.
pub struct Renderer {
    vulkan: Arc<Vulkan>,
    surface: Surface,

    swapchain: Swapchain,
    swapchain_preferences: SwapchainPreferences,
    swapchain_retirement: SwapchainRetirement,

    render_buffer: RenderBuffer,

    line_shader: LinePipeline,
    selection_shader: SelectionPipeline,
    capture_shader: CapturePipeline,

    /// The dynamic state for the renderer, expected to be written to by the main window thread
    /// and read from the render thread.
    pub state: Arc<Mutex<State>>,
}

impl Renderer {
    /// Flag that the swapchain needs to be rebuilt.
    pub fn request_resize(&mut self) {
        self.swapchain.needs_to_rebuild = true;
    }
}

/// Error variants from renderer creation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CreationError {
    /// An allocation failed.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            if let Err(e) = self.vulkan.device().device_wait_idle() {
                error!("Failed to wait for device idle: {e}");
            }

            self.vulkan
                .device()
                .destroy_buffer(self.render_buffer.buffer, None);
            self.vulkan
                .device()
                .free_memory(self.render_buffer.memory, None);

            self.swapchain.destroy(self.vulkan.as_ref(), &self.surface);
        }
    }
}
