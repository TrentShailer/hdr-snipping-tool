use alloc::sync::Arc;

use ash::vk;
use ash_helper::{Swapchain, SwapchainPreferences};
use parking_lot::Mutex;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::{QueuePurpose, Vulkan};

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
    ) -> Result<Self, CreationError> {
        // Create the surface context
        let surface = unsafe { Surface::new(vulkan.as_ref(), display_handle, window_handle)? };

        // Create the swapchain
        let swapchain_preferences = SwapchainPreferences::default()
            .frames_in_flight(3)
            .image_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .format(vec![
                vk::Format::R16G16B16A16_SFLOAT,
                vk::Format::R8G8B8A8_UNORM,
                vk::Format::B8G8R8A8_SNORM,
            ])
            .colour_space(vec![
                vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT,
                vk::ColorSpaceKHR::SRGB_NONLINEAR,
            ]);

        let swapchain = unsafe {
            Swapchain::new(
                vulkan.as_ref(),
                &surface,
                vulkan.transient_pool(),
                vulkan.queue(QueuePurpose::Graphics),
                None,
                &swapchain_preferences,
            )?
        };

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

            state: Arc::new(Mutex::new(State::default())),
        })
    }
}
