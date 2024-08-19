use ash::{
    vk::{CommandBuffer, IndexType, PipelineBindPoint, ShaderStageFlags},
    Device,
};

use crate::renderer::units::{FromPhysical, VkPosition, VkSize};

use super::Border;

impl Border {
    pub fn render(
        &self,
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

        let push_constants = [
            0.0,
            0.0,
            2.0,
            2.0,
            position[0],
            position[1],
            size[0],
            size[1],
            line_size[0],
            line_size[1],
        ];

        let push_constants: Box<[u8]> = push_constants
            .into_iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();

        unsafe {
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                &push_constants,
            );

            device.cmd_draw_indexed(command_buffer, self.indicies, 1, 0, 0, 0);
        }

        Ok(())
    }
}
