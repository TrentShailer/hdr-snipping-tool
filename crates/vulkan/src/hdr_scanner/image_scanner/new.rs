use core::slice;

use ash::vk;
use ash_helper::{create_shader_module_from_spv, try_name, VkError, VulkanContext};

use crate::{hdr_scanner::Error, vulkan::Vulkan};

use super::ImageScanner;

impl ImageScanner {
    pub unsafe fn new(vulkan: &Vulkan) -> Result<Self, Error> {
        // Create descriptor layout
        let descriptor_layout = {
            let bindings = [
                // Image
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                // Buffer
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
            ];

            let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings)
                .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR);

            let layout = unsafe {
                vulkan
                    .device()
                    .create_descriptor_set_layout(&layout_info, None)
            }
            .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?;

            unsafe { try_name(vulkan, layout, "ImageScanner Descriptor Layout") };

            layout
        };

        // Create Pipeline Layout
        let layout = {
            let layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(slice::from_ref(&descriptor_layout));

            let layout = unsafe { vulkan.device().create_pipeline_layout(&layout_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePipelineLayout"))?;

            unsafe { try_name(vulkan, layout, "ImageScanner Layout") };

            layout
        };

        // Create shader module
        let shader = {
            let shader = create_shader_module_from_spv(
                vulkan,
                include_bytes!("../../_shaders/spv/maximum_reduction_image.spv"),
            )?;

            unsafe { try_name(vulkan, shader, "ImageScanner Shader") };

            shader
        };

        // Create pipeline
        let pipeline = {
            let stage_info = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader)
                .name(c"main");

            let create_info = vk::ComputePipelineCreateInfo::default()
                .stage(stage_info)
                .layout(layout);

            let pipeline = unsafe {
                vulkan
                    .device()
                    .create_compute_pipelines(
                        vk::PipelineCache::null(),
                        slice::from_ref(&create_info),
                        None,
                    )
                    .map_err(|e| VkError::new(e.1, "vkCreateComputePipelines"))?[0]
            };

            unsafe { try_name(vulkan, layout, "ImageScanner Pipeline") };

            pipeline
        };

        Ok(Self {
            descriptor_layout,
            layout,
            shader,
            pipeline,
        })
    }
}
