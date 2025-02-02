use core::slice;

use ash::vk;
use ash_helper::VulkanContext;

use crate::{renderer::pipelines::CapturePipeline, Renderer, RendererState};

impl Renderer {
    pub(super) unsafe fn cmd_draw_capture(
        &self,
        command_buffer: vk::CommandBuffer,
        state: RendererState,
    ) {
        let Some(capture) = state.capture else { return };

        self.vulkan.device().cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            self.capture_pipeline.pipeline,
        );

        self.vulkan.device().cmd_bind_vertex_buffers(
            command_buffer,
            0,
            slice::from_ref(&self.render_buffer.buffer),
            slice::from_ref(&self.render_buffer.capture_offset),
        );

        {
            let image_info = vk::DescriptorImageInfo::default()
                .image_layout(vk::ImageLayout::GENERAL)
                .image_view(capture.view);

            let descriptor_write = vk::WriteDescriptorSet::default()
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(slice::from_ref(&image_info));

            self.vulkan
                .push_descriptor_device()
                .cmd_push_descriptor_set(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.capture_pipeline.layout,
                    0,
                    slice::from_ref(&descriptor_write),
                );
        }

        self.vulkan.device().cmd_push_constants(
            command_buffer,
            self.capture_pipeline.layout,
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            0,
            &state.whitepoint.to_ne_bytes(),
        );

        self.vulkan.device().cmd_draw(
            command_buffer,
            CapturePipeline::VERTICIES.len() as u32,
            1,
            0,
            0,
        );
    }
}
