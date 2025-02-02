use ash::vk;
use ash_helper::VulkanContext;

use crate::Vulkan;

mod new;
mod run;

pub struct ImageScanner {
    descriptor_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    shader: vk::ShaderModule,
    pipeline: vk::Pipeline,
}

impl ImageScanner {
    #[inline]
    pub fn output_count(input_extent: vk::Extent2D, subgroup_size: u32) -> u32 {
        let texel_count = input_extent.width * input_extent.height;
        let values_per_thread = 4;

        texel_count
            .div_ceil(values_per_thread)
            .div_ceil(subgroup_size)
    }

    #[inline]
    pub fn dispatch_count(input_extent: vk::Extent2D) -> [u32; 2] {
        let threads_per_dispatch = vk::Extent2D::default().width(16).height(16);

        let x = input_extent.width.div_ceil(threads_per_dispatch.width * 2);
        let y = input_extent
            .height
            .div_ceil(threads_per_dispatch.height * 2);

        [x, y]
    }
}

impl ImageScanner {
    pub unsafe fn destroy(&self, vulkan: &Vulkan) {
        vulkan.device().destroy_pipeline(self.pipeline, None);
        vulkan.device().destroy_pipeline_layout(self.layout, None);
        vulkan.device().destroy_shader_module(self.shader, None);
        vulkan
            .device()
            .destroy_descriptor_set_layout(self.descriptor_layout, None);
    }
}
