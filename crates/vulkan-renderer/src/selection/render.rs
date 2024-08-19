use ash::{
    vk::{CommandBuffer, IndexType, PipelineBindPoint, ShaderStageFlags},
    Device,
};

use crate::renderer::units::{FromPhysical, VkPosition, VkSize};

use super::Selection;

impl Selection {
    pub fn render(
        &self,
        device: &Device,
        command_buffer: CommandBuffer,
        selection_top_left: [u32; 2],
        selection_size: [u32; 2],
        window_size: [u32; 2],
        window_scale: f64,
    ) -> Result<(), ash::vk::Result> {
        let selection_top_left = VkPosition::from_physical(selection_top_left, window_size);
        let selection_size = VkSize::from_physical(selection_size, window_size);
        let selection_position = VkPosition::get_center(selection_top_left, selection_size);

        let target_position = selection_position.as_f32_array();
        let target_size = selection_size.as_f32_array();

        let push_constants = [
            target_position[0],
            target_position[1],
            target_size[0],
            target_size[1],
        ];
        let push_constants: Box<[u8]> = push_constants
            .into_iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();

        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.shading_pipeline,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_push_constants(
                command_buffer,
                self.shading_pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                &push_constants,
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
