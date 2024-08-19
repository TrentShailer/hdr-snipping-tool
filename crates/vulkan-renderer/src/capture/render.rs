use ash::{
    vk::{CommandBuffer, IndexType, PipelineBindPoint, ShaderStageFlags},
    Device,
};

use super::Capture;

impl Capture {
    pub fn render(
        &self,
        device: &Device,
        command_buffer: CommandBuffer,
    ) -> Result<(), ash::vk::Result> {
        if !self.loaded {
            return Ok(());
        }

        unsafe {
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
            );
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ShaderStageFlags::FRAGMENT,
                0,
                &self.whitepoint.to_le_bytes(),
            );

            device.cmd_draw_indexed(command_buffer, self.indicies, 1, 0, 0, 0);
        }

        Ok(())
    }
}
