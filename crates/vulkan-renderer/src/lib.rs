mod new;
mod objects;
mod pipelines;
mod render;
mod swapchain;
mod units;

use std::sync::Arc;

use ash::vk::{
    CommandBuffer, DescriptorSetLayout, Fence, Image, ImageView, Pipeline, PipelineLayout,
    Semaphore, ShaderModule, SwapchainKHR, Viewport,
};
use hdr_capture::HdrCapture;
use objects::{Capture, MouseGuides, Selection};
use thiserror::Error;
use tracing::{instrument, Level};
use vulkan_instance::{VulkanError, VulkanInstance};

/// Renderer for HdrSnippingTool
pub struct Renderer {
    vk: Arc<VulkanInstance>,

    recreate_swapchain: bool,
    command_buffers: Box<[(CommandBuffer, Fence)]>,
    acquire_fences: Vec<Fence>,
    render_semaphores: Vec<Semaphore>,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: SwapchainKHR,

    attachment_images: Vec<Image>,
    attachment_views: Vec<ImageView>,

    viewport: Viewport,

    capture: Capture,
    selection: Selection,
    mouse_guides: MouseGuides,

    pipeline_layouts: Vec<PipelineLayout>,
    pipelines: Vec<Pipeline>,
    shaders: Vec<ShaderModule>,
    descriptor_layouts: Vec<DescriptorSetLayout>,
}

impl Renderer {
    /// Flags that the swapchain needs to be recreated
    pub fn queue_recreate_swapchain(&mut self) {
        self.recreate_swapchain = true;
    }

    /// Loads a capture into the renderer
    #[instrument("Renderer::load_capture", level = Level::DEBUG, skip_all, err)]
    pub fn load_capture(&mut self, capture: &HdrCapture) -> Result<(), Error> {
        self.capture.load_capture(capture)
    }

    /// Unloads a capture from the renderer
    pub fn unload_capture(&mut self) {
        self.capture.unload_capture()
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Vulkan(#[from] VulkanError),
}
