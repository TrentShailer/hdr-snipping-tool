pub mod plane;
pub mod renderer;
pub mod text;
pub mod vertex;

use std::sync::Arc;

use vertex::Vertex;
use vulkan_instance::texture::Texture;
use vulkano::{
    buffer::Subbuffer,
    descriptor_set::PersistentDescriptorSet,
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{Framebuffer, RenderPass},
    swapchain::Swapchain,
    sync::GpuFuture,
};

pub struct Renderer {
    pub texture: Option<Arc<Texture>>,
    pub texture_ds: Option<Arc<PersistentDescriptorSet>>,
    pub recreate_swapchain: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub render_pass: Arc<RenderPass>,
    pub viewport: Viewport,
    pub pipeline: Arc<GraphicsPipeline>,
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
}
