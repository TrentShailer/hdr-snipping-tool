use std::sync::Arc;

use ash::{
    vk::{
        Buffer, CommandBuffer, DeviceMemory, IndexType, Pipeline, PipelineBindPoint,
        PipelineLayout, ShaderStageFlags,
    },
    Device,
};
use bytemuck::bytes_of;
use tracing::{instrument, Level};
use vulkan_instance::VulkanInstance;

use crate::{
    objects::Border,
    pipelines::{
        selection_shading::{PushConstants, Vertex},
        vertex_index_buffer::create_vertex_and_index_buffer,
    },
    units::{FromPhysical, VkPosition, VkSize},
};

use hdr_capture::Selection as SelectionArea;

const LOCKED_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

/* Vertex indicies
    .0				.2
        .1		.3

        .7		.5
    .6				.4
*/

pub struct Selection {
    vk: Arc<VulkanInstance>,

    border: Border,

    vertex_buffer: (Buffer, DeviceMemory),
    index_buffer: (Buffer, DeviceMemory),
    indicies: u32,

    shading_pipeline: Pipeline,
    shading_pipeline_layout: PipelineLayout,

    push_constants: PushConstants,
}

impl Selection {
    #[instrument("Selection::new", level = Level::DEBUG, skip_all, err)]
    pub fn new(
        vk: Arc<VulkanInstance>,
        shading_pipeline: Pipeline,
        shading_pipeline_layout: PipelineLayout,
        border_pipeline: Pipeline,
        border_pipeline_layout: PipelineLayout,
    ) -> Result<Self, crate::Error> {
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
            create_vertex_and_index_buffer(&vk, &verticies, &indicies)?;

        let border = Border::new(
            vk.clone(),
            border_pipeline,
            border_pipeline_layout,
            [255, 255, 255, 255],
            2.0,
        )?;

        let push_constants = PushConstants {
            base_position: [0.0, 0.0],
            base_size: [1.0, 1.0],
            target_position: [0.0, 0.0],
            target_size: [1.0, 1.0],
        };

        Ok(Self {
            vk,

            border,

            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            shading_pipeline,
            shading_pipeline_layout,

            push_constants,
        })
    }

    #[instrument("Selection::render", level = Level::DEBUG, skip_all, err)]
    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: CommandBuffer,
        selection: SelectionArea,
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), ash::vk::Result> {
        let selection_top_left = VkPosition::from_physical(selection.left_top(), window_size);
        let selection_size = VkSize::from_physical(selection.size(), window_size);
        let selection_position = VkPosition::get_center(selection_top_left, selection_size);

        let target_position = selection_position.as_f32_array();
        let target_size = selection_size.as_f32_array();

        self.push_constants.target_position = target_position;
        self.push_constants.target_size = target_size;

        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.shading_pipeline,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_push_constants(
                command_buffer,
                self.shading_pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                bytes_of(&self.push_constants),
            );

            device.cmd_draw_indexed(command_buffer, self.indicies, 1, 0, 0, 0);
        }

        self.border.render(
            device,
            command_buffer,
            selection_position,
            selection_size,
            window_size,
            window_scale,
        )?;

        Ok(())
    }
}

impl Drop for Selection {
    fn drop(&mut self) {
        unsafe {
            self.vk.device.destroy_buffer(self.vertex_buffer.0, None);
            self.vk.device.free_memory(self.vertex_buffer.1, None);
            self.vk.device.destroy_buffer(self.index_buffer.0, None);
            self.vk.device.free_memory(self.index_buffer.1, None);
        }
    }
}
