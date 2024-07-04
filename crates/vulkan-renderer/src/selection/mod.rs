pub mod render;

use std::sync::Arc;

use vulkan_instance::VulkanInstance;
use vulkano::{buffer::Subbuffer, pipeline::GraphicsPipeline};

use crate::{
    border::Border, pipelines::selection_shading::Vertex,
    vertex_index_buffer::create_vertex_and_index_buffer,
};

const LOCKED_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

/* Vertex indicies
    .0				.2
        .1		.3

        .7		.5
    .6				.4
*/

pub struct Selection {
    pub border: Border,
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub shading_pipeline: Arc<GraphicsPipeline>,
}

impl Selection {
    pub fn new(
        vk: &VulkanInstance,
        shading_pipeline: Arc<GraphicsPipeline>,
        border_pipeline: Arc<GraphicsPipeline>,
    ) -> Result<Self, crate::vertex_index_buffer::Error> {
        let color = [0, 0, 0, 127];
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                color,
                flags: LOCKED_FLAG,
            }, // TL
            Vertex {
                position: [-0.5, -0.5],
                color,
                flags: NO_FLAGS,
            }, // CTL
            Vertex {
                position: [1.0, -1.0],
                color,
                flags: LOCKED_FLAG,
            }, // TR
            Vertex {
                position: [0.5, -0.5],
                color,
                flags: NO_FLAGS,
            }, // CTR
            Vertex {
                position: [1.0, 1.0],
                color,
                flags: LOCKED_FLAG,
            }, // BR
            Vertex {
                position: [0.5, 0.5],
                color,
                flags: NO_FLAGS,
            }, // CBR
            Vertex {
                position: [-1.0, 1.0],
                color,
                flags: LOCKED_FLAG,
            }, // BL
            Vertex {
                position: [-0.5, 0.5],
                color,
                flags: NO_FLAGS,
            }, // CBL
        ];

        let indicies = vec![
            0, 2, 1, 1, 2, 3, //
            3, 2, 5, 4, 5, 2, //
            7, 5, 4, 6, 7, 4, //
            1, 7, 6, 0, 1, 6, //
        ];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        let border = Border::new(vk, border_pipeline, [255, 255, 255, 255], 2.0)?;

        Ok(Self {
            border,
            vertex_buffer,
            index_buffer,
            shading_pipeline,
        })
    }
}
