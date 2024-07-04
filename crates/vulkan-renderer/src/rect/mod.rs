pub mod render;

use std::sync::Arc;

use vulkan_instance::VulkanInstance;
use vulkano::{buffer::Subbuffer, pipeline::GraphicsPipeline};

use crate::{pipelines::rect::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

pub struct Rect {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
}

impl Rect {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        color: [u8; 4],
    ) -> Result<Self, crate::vertex_index_buffer::Error> {
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                color,
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                color,
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                color,
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                color,
            }, // BL
        ];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            pipeline,
        })
    }
}
