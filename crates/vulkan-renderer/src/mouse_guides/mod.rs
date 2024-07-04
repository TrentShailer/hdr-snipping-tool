pub mod render;

use std::sync::Arc;

use vulkan_instance::VulkanInstance;
use vulkano::{buffer::Subbuffer, pipeline::GraphicsPipeline};

use crate::{pipelines::mouse_guides::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

const LEFT_OR_TOP_FLAG: u32 = 0b00000000_00000000_00000000_00000100;
const POSITIVE_SHIFT_FLAG: u32 = 0b00000000_00000000_00000000_00000010;
const VERTICAL_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

/* Vertex indicies
            .0.1

    .6					.4
    .7					.5

            .2.3
*/

pub struct MouseGuides {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub line_size: f32,
}

impl MouseGuides {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        line_size: f32,
    ) -> Result<Self, crate::vertex_index_buffer::Error> {
        let color = [255, 255, 255, 255];
        let verticies = vec![
            Vertex {
                position: [0.0, -1.0],
                color,
                flags: LEFT_OR_TOP_FLAG | VERTICAL_FLAG,
            },
            Vertex {
                position: [0.0, -1.0],
                color,
                flags: LEFT_OR_TOP_FLAG | POSITIVE_SHIFT_FLAG | VERTICAL_FLAG,
            },
            Vertex {
                position: [0.0, 1.0],
                color,
                flags: VERTICAL_FLAG,
            },
            Vertex {
                position: [0.0, 1.0],
                color,
                flags: POSITIVE_SHIFT_FLAG | VERTICAL_FLAG,
            },
            //
            Vertex {
                position: [1.0, 0.0],
                color,
                flags: NO_FLAGS,
            },
            Vertex {
                position: [1.0, 0.0],
                color,
                flags: POSITIVE_SHIFT_FLAG,
            },
            Vertex {
                position: [-1.0, 0.0],
                color,
                flags: LEFT_OR_TOP_FLAG,
            },
            Vertex {
                position: [-1.0, 0.0],
                color,
                flags: LEFT_OR_TOP_FLAG | POSITIVE_SHIFT_FLAG,
            },
        ];

        let indicies = vec![
            0, 1, 2, 2, 1, 3, //
            4, 5, 6, 6, 5, 7, //
        ];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            line_size,
        })
    }
}
