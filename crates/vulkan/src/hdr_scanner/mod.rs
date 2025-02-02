use alloc::sync::Arc;
use ash::vk;
use ash_helper::{AllocationError, VkError, VulkanContext};
use buffer_scanner::BufferScanner;
use image_scanner::ImageScanner;
use resources::Resources;
use thiserror::Error;
use tracing::error;

use crate::{QueuePurpose, Vulkan};

mod buffer_scanner;
mod image_scanner;
mod new;
mod resources;
mod run;

pub const COMMAND_BUFFERS: usize = 5;

/// Scans an HDR capture for if it contains HDR Content.
pub struct HdrScanner {
    vulkan: Arc<Vulkan>,

    command_objects: Vec<(vk::CommandPool, vk::CommandBuffer)>,

    semaphore: vk::Semaphore,
    semaphore_value: u64,

    host_memory: vk::DeviceMemory,
    host_buffer: vk::Buffer,

    buffer_scanner: BufferScanner,
    image_scanner: ImageScanner,

    resources: Option<Resources>,
}

impl HdrScanner {
    /// Prepare the resources required for scanning.
    pub unsafe fn prepare(&mut self, extent: vk::Extent2D) -> Result<(), Error> {
        self.free_resources();

        self.resources = Some(Resources::new(&self.vulkan, extent)?);

        Ok(())
    }

    /// Free the resources acquired for scanning.
    pub unsafe fn free_resources(&mut self) {
        if let Some(resources) = self.resources.take() {
            resources.destory(&self.vulkan);
        }
    }
}

impl Drop for HdrScanner {
    fn drop(&mut self) {
        unsafe {
            let queue = self.vulkan.queue(QueuePurpose::Compute).lock();
            if let Err(e) = self.vulkan.device().queue_wait_idle(*queue) {
                error!("Failed to wait for device idle while dropping HdrScanner:\n{e}");
            };

            self.buffer_scanner.destroy(&self.vulkan);
            self.image_scanner.destroy(&self.vulkan);

            self.vulkan.device().destroy_buffer(self.host_buffer, None);
            self.vulkan.device().free_memory(self.host_memory, None);

            for (pool, _) in &self.command_objects {
                self.vulkan.device().destroy_command_pool(*pool, None);
            }

            self.vulkan.device().destroy_semaphore(self.semaphore, None);

            drop(queue);

            self.free_resources();
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
