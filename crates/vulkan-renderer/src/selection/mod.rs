pub mod drop;
pub mod render;

use ash::vk::{Buffer, DeviceMemory, Pipeline, PipelineLayout};
use vulkan_instance::VulkanInstance;

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

    pub vertex_buffer: (Buffer, DeviceMemory),
    pub index_buffer: (Buffer, DeviceMemory),
    pub indicies: u32,

    pub shading_pipeline: Pipeline,
    pub shading_pipeline_layout: PipelineLayout,
}

impl Selection {
    pub fn new(
        vk: &VulkanInstance,
        shading_pipeline: Pipeline,
        shading_pipeline_layout: PipelineLayout,
        border_pipeline: Pipeline,
        border_pipeline_layout: PipelineLayout,
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
            2, 4, 3, 3, 4, 5, //
            4, 6, 5, 5, 6, 7, //
            6, 0, 7, 7, 0, 1, //
        ];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, &verticies, &indicies)?;

        let border = Border::new(
            vk,
            border_pipeline,
            border_pipeline_layout,
            [255, 255, 255, 255],
            2.0,
        )?;

        Ok(Self {
            border,

            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            shading_pipeline,
            shading_pipeline_layout,
        })
    }
}
