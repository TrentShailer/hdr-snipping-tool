use ash::{
    vk::{
        Buffer, CommandBuffer, DeviceMemory, IndexType, Pipeline, PipelineBindPoint,
        PipelineLayout, ShaderStageFlags,
    },
    Device,
};
use bytemuck::bytes_of;
use vulkan_instance::VulkanInstance;

use crate::{
    pipelines::{
        mouse_guides::{PushConstants, Vertex},
        vertex_index_buffer::create_vertex_and_index_buffer,
    },
    units::{FromPhysical, VkPosition, VkSize},
};

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

pub struct MouseGuides<'d> {
    device: &'d Device,

    vertex_buffer: (Buffer, DeviceMemory),
    index_buffer: (Buffer, DeviceMemory),
    indicies: u32,

    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,

    push_constants: PushConstants,
    line_size: f32,
}

impl<'d> MouseGuides<'d> {
    pub fn new(
        vk: &'d VulkanInstance,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        line_size: f32,
    ) -> Result<Self, crate::Error> {
        let color = [128, 128, 128, 64];
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
            create_vertex_and_index_buffer(vk, &verticies, &indicies)?;

        let push_constants = PushConstants {
            mouse_position: [0.0, 0.0],
            line_size: [0.0, 0.0],
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
        mouse_position: [u32; 2],
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), ash::vk::Result> {
        let mouse_position = VkPosition::from_physical(mouse_position, window_size);

        let line_size =
            VkSize::from_physical([self.line_size, self.line_size], window_size) * window_scale;

        let mouse_position = mouse_position.as_f32_array();
        let line_size = line_size.as_f32_array();
        self.push_constants.line_size = line_size;
        self.push_constants.mouse_position = mouse_position;

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

impl<'d> Drop for MouseGuides<'d> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.vertex_buffer.0, None);
            self.device.free_memory(self.vertex_buffer.1, None);
            self.device.destroy_buffer(self.index_buffer.0, None);
            self.device.free_memory(self.index_buffer.1, None);
        }
    }
}
