use alloc::sync::Arc;

use ash::vk;
use ash_helper::{Swapchain, SwapchainPreferences, SwapchainRetirement};
use parking_lot::Mutex;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use tracing::debug;

use crate::Vulkan;

use super::{
    CreationError, Renderer, State,
    buffer::RenderBuffer,
    context::Surface,
    pipelines::{CapturePipeline, LinePipeline, SelectionPipeline},
};

impl Renderer {
    /// Create a new instance of the renderer.
    pub unsafe fn new(
        vulkan: Arc<Vulkan>,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        state: Arc<Mutex<State>>,
    ) -> Result<Self, CreationError> {
        // Create the surface context
        let surface = unsafe { Surface::new(&vulkan, display_handle, window_handle)? };

        // Create the Swapchain
        let swapchain_preferences = SwapchainPreferences::default()
            .image_count(3)
            .format(vec![
                vk::Format::R16G16B16A16_SFLOAT,
                vk::Format::R8G8B8A8_UNORM,
                vk::Format::B8G8R8A8_SNORM,
            ])
            .colour_space(vec![
                vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
                vk::ColorSpaceKHR::SRGB_NONLINEAR,
            ])
            .present_mode(vec![vk::PresentModeKHR::FIFO]);

        let swapchain = {
            let create_info =
                swapchain_preferences.get_swapchain_create_info(vulkan.as_ref(), &surface)?;

            unsafe { Swapchain::new(vulkan.as_ref(), &surface, None, create_info)? }
        };
        debug!("Created {swapchain:?}");

        let swapchain_retirement = SwapchainRetirement::new();

        // Create an initialise the render Vertex/Index/Instance buffer.
        let buffer = RenderBuffer::new(&vulkan)?;

        // Create the pipelines
        let line_shader = unsafe { LinePipeline::new(Arc::clone(&vulkan))? };
        let selection_shader = unsafe { SelectionPipeline::new(Arc::clone(&vulkan))? };
        let capture_shader = unsafe { CapturePipeline::new(Arc::clone(&vulkan))? };

        Ok(Self {
            vulkan,
            surface,

            render_buffer: buffer,

            line_shader,
            selection_shader,
            capture_shader,

            swapchain,
            swapchain_preferences,
            swapchain_retirement,

            state,
        })
    }
}
