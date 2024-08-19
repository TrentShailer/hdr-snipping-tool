pub mod drop;
pub mod render;

use ash::vk::{Buffer, DeviceMemory, Pipeline, PipelineLayout};
use vulkan_instance::VulkanInstance;

use crate::{pipelines::border::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

/* Vertex indicies
    .0				.2
        .1		.3

        .7		.5
    .6				.4
*/

const OUTER_FLAG: u32 = 0b00000000_00000000_00000000_00000100;
const TOP_FLAG: u32 = 0b00000000_00000000_00000000_00000010;
const LEFT_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

pub struct Border {
    pub vertex_buffer: (Buffer, DeviceMemory),
    pub index_buffer: (Buffer, DeviceMemory),
    pub indicies: u32,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: Pipeline,

    pub line_size: f32,
}

impl Border {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        color: [u8; 4],
        line_size: f32,
    ) -> Result<Self, crate::vertex_index_buffer::Error> {
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                color,
                flags: OUTER_FLAG | TOP_FLAG | LEFT_FLAG,
            }, // TL
            Vertex {
                position: [-1.0, -1.0],
                color,
                flags: TOP_FLAG | LEFT_FLAG,
            }, // CTL
            Vertex {
                position: [1.0, -1.0],
                color,
                flags: OUTER_FLAG | TOP_FLAG,
            }, // TR
            Vertex {
                position: [1.0, -1.0],
                color,
                flags: TOP_FLAG,
            }, // CTR
            Vertex {
                position: [1.0, 1.0],
                color,
                flags: OUTER_FLAG,
            }, // BR
            Vertex {
                position: [1.0, 1.0],
                color,
                flags: NO_FLAGS,
            }, // CBR
            Vertex {
                position: [-1.0, 1.0],
                color,
                flags: OUTER_FLAG | LEFT_FLAG,
            }, // BL
            Vertex {
                position: [-1.0, 1.0],
                color,
                flags: LEFT_FLAG,
            }, // CBL
        ];

        let indicies = vec![
            0, 2, 1, 1, 2, 3, //
            3, 2, 5, 4, 5, 2, //
            7, 5, 4, 6, 7, 4, //
            1, 7, 6, 0, 1, 6, //
        ];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, &verticies, &indicies)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            pipeline_layout,
            pipeline,

            line_size,
        })
    }
}
