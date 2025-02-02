use ash::vk;
use ash_helper::VulkanContext;
use bytemuck::{Pod, Zeroable};

use crate::Vulkan;

mod new;
mod run;

pub struct BufferScanner {
    descriptor_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    shader: vk::ShaderModule,
    pipeline: vk::Pipeline,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct PushConstants {
    pub input_length: u32,
}

impl BufferScanner {
    #[inline]
    pub fn output_count(input_count: u32, subgroup_size: u32) -> u32 {
        let subgroups_per_dispatch = 128 / subgroup_size;

        let consumed_per_dispatch = 128 * subgroup_size;
        let produced_per_dispatch = subgroups_per_dispatch;

        let number_of_dispatches = input_count as f64 / consumed_per_dispatch as f64;

        (number_of_dispatches * produced_per_dispatch as f64).ceil() as u32
    }

    #[inline]
    pub fn dispatch_count(input_count: u32, subgroup_size: u32) -> u32 {
        let values_processed_per_dispatch = 128 * subgroup_size;
        input_count.div_ceil(values_processed_per_dispatch)
    }
}

impl BufferScanner {
    pub unsafe fn destroy(&self, vulkan: &Vulkan) {
        vulkan.device().destroy_pipeline(self.pipeline, None);
        vulkan.device().destroy_pipeline_layout(self.layout, None);
        vulkan.device().destroy_shader_module(self.shader, None);
        vulkan
            .device()
            .destroy_descriptor_set_layout(self.descriptor_layout, None);
    }
}
