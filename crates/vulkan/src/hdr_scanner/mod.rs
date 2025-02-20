use alloc::sync::Arc;

use ash::vk;
use ash_helper::{AllocationError, VkError, VulkanContext};
use thiserror::Error;

use crate::Vulkan;

mod new;
mod run;

/// Scans an `HdrImage` to find the value of the brightest colour component.
pub struct HdrScanner {
    vulkan: Arc<Vulkan>,

    descriptor_layout: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    shader: vk::ShaderModule,
    pipeline: vk::Pipeline,

    semaphore: vk::Semaphore,
    semaphore_value: u64,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    // STORAGE_BUFFER, TRANSFER_SRC, TRANSFER_DST
    buffer: vk::Buffer,
    // DEVICE_LOCAL, 4 B
    memory: vk::DeviceMemory,

    // TRANSFER_DST
    staging_buffer: vk::Buffer,
    // HOST_COHERENT, HOST_VISIBLE, 4 B
    staging_memory: vk::DeviceMemory,
}

impl Drop for HdrScanner {
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

/// Error variants from creating the HDR Scanner.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HdrScannerError {
    /// A Vulkan allocation failed.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
