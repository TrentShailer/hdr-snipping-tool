pub mod load;
pub mod render;

use std::sync::Arc;

use scrgb_tonemapper::tonemap_output::TonemapOutput;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::Subbuffer, descriptor_set::PersistentDescriptorSet, pipeline::GraphicsPipeline,
};

use crate::{pipelines::capture::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

pub struct Capture {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub capture: Option<Arc<TonemapOutput>>,
    pub capture_ds: Option<Arc<PersistentDescriptorSet>>,
}

impl Capture {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
    ) -> Result<Self, crate::vertex_index_buffer::Error> {
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 0.0],
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                uv: [1.0, 0.0],
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, 1.0],
            }, // BL
        ];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            capture: None,
            capture_ds: None,
        })
    }
}
