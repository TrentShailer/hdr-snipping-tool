use alloc::sync::Arc;
use core::slice;

use ash::{ext, vk};
use ash_helper::{Context, VkError, VulkanContext, try_name, try_name_all};

use crate::{Vulkan, shaders::tonemap_hdr_to_sdr};

use super::{HdrToSdrTonemapper, TonemapperError};

impl HdrToSdrTonemapper {
    /// Creates a new instance of an HDR to SDR Tonemapper.
    pub fn new(vulkan: Arc<Vulkan>) -> Result<Self, TonemapperError> {
        // Descriptor layouts
        let descriptor_layouts = {
            let layouts = unsafe {
                tonemap_hdr_to_sdr::set_layouts(
                    vulkan.device(),
                    vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
                )
                .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?
            };

            unsafe {
                try_name_all(
                    vulkan.as_ref(),
                    &layouts,
                    "HDR to SDR Tonemapper Descriptor Layout",
                )
            };

            layouts
        };

        // Pipeline layout
        let pipeline_layout = {
            let push_range = tonemap_hdr_to_sdr::PushConstants::push_constant_range();

            let create_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(slice::from_ref(&push_range));

            let layout = unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePiplineLayout"))?;

            unsafe {
                try_name(
                    vulkan.as_ref(),
                    layout,
                    "HDR to SDR Tonemapper Pipeline Layout",
                )
            };

            layout
        };

        // Shader
        let shader = {
            let push_range = tonemap_hdr_to_sdr::PushConstants::push_constant_range();

            let create_info = vk::ShaderCreateInfoEXT::default()
                .code(tonemap_hdr_to_sdr::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(tonemap_hdr_to_sdr::compute_main::STAGE)
                .name(tonemap_hdr_to_sdr::compute_main::ENTRY_POINT)
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(slice::from_ref(&push_range));

            let device: &ext::shader_object::Device = unsafe { vulkan.context() };
            let shaders = unsafe { device.create_shaders(slice::from_ref(&create_info), None) }
                .map_err(|(_, e)| VkError::new(e, "vkCreateShadersEXT"))?;

            let shader = shaders[0];
            unsafe { try_name(vulkan.as_ref(), shader, "TONEMAPPER COMPUTE SHADER") };

            shader
        };

        Ok(Self {
            vulkan,

            descriptor_layouts,
            pipeline_layout,
            shader,
        })
    }
}
