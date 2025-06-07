use ash::{ext, vk};
use ash_helper::{AllocationError, Context, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext};
use thiserror::Error;

use crate::Vulkan;

mod new;
mod run;

/// Scans an `HdrImage` to find the value of the brightest colour component.
pub struct HdrScanner<'vulkan> {
    vulkan: &'vulkan Vulkan,

    descriptor_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    shader: vk::ShaderEXT,

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

impl Drop for HdrScanner<'_> {
    fn drop(&mut self) {
        unsafe {
            let shader_device: &ext::shader_object::Device = self.vulkan.context();
            shader_device.destroy_shader(self.shader, VK_GLOBAL_ALLOCATOR.as_deref());

            self.vulkan
                .device()
                .destroy_pipeline_layout(self.pipeline_layout, VK_GLOBAL_ALLOCATOR.as_deref());

            self.descriptor_layouts.iter().for_each(|layout| {
                self.vulkan
                    .device()
                    .destroy_descriptor_set_layout(*layout, VK_GLOBAL_ALLOCATOR.as_deref());
            });

            self.vulkan
                .device()
                .destroy_buffer(self.buffer, VK_GLOBAL_ALLOCATOR.as_deref());
            self.vulkan
                .device()
                .destroy_buffer(self.staging_buffer, VK_GLOBAL_ALLOCATOR.as_deref());

            self.vulkan
                .device()
                .free_memory(self.memory, VK_GLOBAL_ALLOCATOR.as_deref());
            self.vulkan
                .device()
                .free_memory(self.staging_memory, VK_GLOBAL_ALLOCATOR.as_deref());

            self.vulkan
                .device()
                .destroy_command_pool(self.command_pool, VK_GLOBAL_ALLOCATOR.as_deref());
            self.vulkan
                .device()
                .destroy_semaphore(self.semaphore, VK_GLOBAL_ALLOCATOR.as_deref());
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
