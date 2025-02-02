use alloc::sync::Arc;

use ash::vk;
use ash_helper::{AllocationError, VkError, VulkanContext};
use thiserror::Error;

use crate::Vulkan;

mod new;
mod run;

/// The number of bins in the histogram.
pub const BIN_COUNT: u64 = 256;

/// Generates a histogram with `BIN_COUNT` bins for a `HdrImage`.
pub struct HistogramGenerator {
    vulkan: Arc<Vulkan>,

    descriptor_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    shader: vk::ShaderModule,
    pipeline: vk::Pipeline,

    semaphore: vk::Semaphore,
    semaphore_value: u64,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    buffer: vk::Buffer,
    memory: vk::DeviceMemory,

    staging_buffer: vk::Buffer,
    staging_memory: vk::DeviceMemory,
}

impl Drop for HistogramGenerator {
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

            self.vulkan.device().destroy_buffer(self.buffer, None);
            self.vulkan
                .device()
                .destroy_buffer(self.staging_buffer, None);

            self.vulkan.device().free_memory(self.memory, None);
            self.vulkan.device().free_memory(self.staging_memory, None);

            self.vulkan
                .device()
                .destroy_command_pool(self.command_pool, None);
            self.vulkan.device().destroy_semaphore(self.semaphore, None);
        }
    }
}

/// Error variants from using image maximum.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A Vulkan allocation failed.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
