use alloc::sync::Arc;

use ash::{ext, vk};
use ash_helper::{AllocationError, Context, VkError, VulkanContext};
use thiserror::Error;

use crate::Vulkan;

mod new;
mod run;

/// Performs tonemapping on an HDR image in `R16G16B16A16_SFLOAT` format to produce an SDR image in
/// `R8G8B8A8_UNORM` that matches the original reasonably closely.
pub struct HdrToSdrTonemapper {
    vulkan: Arc<Vulkan>,

    descriptor_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    shader: vk::ShaderEXT,
}

impl Drop for HdrToSdrTonemapper {
    fn drop(&mut self) {
        unsafe {
            let shader_device: &ext::shader_object::Device = self.vulkan.context();
            shader_device.destroy_shader(self.shader, None);
            self.vulkan
                .device()
                .destroy_pipeline_layout(self.pipeline_layout, None);

            self.descriptor_layouts.iter().for_each(|layout| {
                self.vulkan
                    .device()
                    .destroy_descriptor_set_layout(*layout, None);
            });
        }
    }
}

/// Tonemapper error variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TonemapperError {
    /// An allocation failed, likely from the output image.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
