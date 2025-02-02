use alloc::sync::Arc;

use ash::vk;
use ash_helper::{AllocationError, VkError, VulkanContext};
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

use crate::Vulkan;

mod new;
mod run;

/// Performs tonemapping on an HDR image in `R16G16B16A16_SFLOAT` format to produce an SDR image in
/// `R8G8B8A8_UNORM` that matches the original reasonably closely.
pub struct HdrToSdrTonemapper {
    vulkan: Arc<Vulkan>,

    descriptor_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    shader: vk::ShaderModule,
    pipeline: vk::Pipeline,
}

/// The shader Push Constants.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy)]
pub struct PushConstants {
    /// The maximum brightness a color component is clamped to.
    pub whitepoint: f32,
}

impl Drop for HdrToSdrTonemapper {
    fn drop(&mut self) {
        unsafe {
            self.vulkan.device().destroy_pipeline(self.pipeline, None);
            self.vulkan
                .device()
                .destroy_shader_module(self.shader, None);
            self.vulkan
                .device()
                .destroy_pipeline_layout(self.layout, None);
            self.vulkan
                .device()
                .destroy_descriptor_set_layout(self.descriptor_layout, None);
        }
    }
}

/// Tonemapper error variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An allocation failed, likely from the output image.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
