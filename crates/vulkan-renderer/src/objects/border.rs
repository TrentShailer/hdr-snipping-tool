use ash::{
    vk::{
        Buffer, CommandBuffer, DeviceMemory, IndexType, Pipeline, PipelineBindPoint,
        PipelineLayout, ShaderStageFlags,
    },
    Device,
};
use bytemuck::bytes_of;
use tracing::instrument;
use vulkan_instance::VulkanInstance;

use crate::{
    pipelines::{
        border::{PushConstants, Vertex},
        vertex_index_buffer::create_vertex_and_index_buffer,
    },
    units::{FromPhysical, VkPosition, VkSize},
};

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

pub struct Border<'d> {
    device: &'d Device,

    vertex_buffer: (Buffer, DeviceMemory),
    index_buffer: (Buffer, DeviceMemory),
    indicies: u32,

    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,

    push_constants: PushConstants,
    line_size: f32,
}

impl<'d> Border<'d> {
    #[instrument("Border::new", skip_all, err)]
    pub fn new(
        vk: &'d VulkanInstance,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        color: [u8; 4],
        line_size: f32,
    ) -> Result<Self, crate::Error> {
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

        let push_constants = PushConstants {
            base_position: [0.0, 0.0],
            base_size: [2.0, 2.0],
            target_position: [0.0, 0.0],
            target_size: [2.0, 2.0],
            line_size: [line_size, line_size],
        };

        Ok(Self {
            device: &vk.device,

            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            pipeline_layout,
            pipeline,

            push_constants,
            line_size,
        })
    }

    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: CommandBuffer,
        position: VkPosition,
        size: VkSize,
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), ash::vk::Result> {
        let line_size =
            VkSize::from_physical([self.line_size, self.line_size], window_size) * window_scale;

        let position = position.as_f32_array();
        let size = size.as_f32_array();
        let line_size = line_size.as_f32_array();

        self.push_constants.target_position = position;
        self.push_constants.target_size = size;
        self.push_constants.line_size = line_size;

        unsafe {
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                bytes_of(&self.push_constants),
            );

            device.cmd_draw_indexed(command_buffer, self.indicies, 1, 0, 0, 0);
        }

        Ok(())
    }
}

impl<'d> Drop for Border<'d> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.vertex_buffer.0, None);
            self.device.free_memory(self.vertex_buffer.1, None);
            self.device.destroy_buffer(self.index_buffer.0, None);
            self.device.free_memory(self.index_buffer.1, None);
        }
    }
}
