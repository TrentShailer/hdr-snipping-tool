use core::slice;

use ash::vk;
use ash_helper::{
    allocate_buffer, cmd_transition_image, onetime_command, AllocationError, VkError, VulkanContext,
};
use thiserror::Error;

use crate::{QueuePurpose, Vulkan};

#[derive(Clone, Copy)]
/// An SDR image in `R8G8B8A8_UNORM` format.
pub struct SdrImage {
    /// The Vulkan image.
    pub image: vk::Image,

    /// The image memory.
    pub memory: vk::DeviceMemory,

    /// The image extent.
    pub extent: vk::Extent2D,
}

impl SdrImage {
    /// Copy the image to a slice in CPU memory.
    pub unsafe fn copy_to_cpu(&self, vulkan: &Vulkan) -> Result<Vec<u8>, Error> {
        // Create staging
        let (staging_buffer, staging_memory) = {
            let queue_family = vulkan.queue_family_index();

            let buffer_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(&queue_family))
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .size(self.extent.width as u64 * self.extent.height as u64 * 4);

            let (buffer, memory, _) = unsafe {
                allocate_buffer(
                    vulkan,
                    &buffer_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "SDR to CPU Staging",
                )?
            };

            (buffer, memory)
        };

        // Copy tonemapped image to staging
        unsafe {
            onetime_command(
                vulkan,
                vulkan.transient_pool(),
                vulkan.queue(QueuePurpose::Compute),
                |vk, command_buffer| {
                    cmd_transition_image(
                        vk,
                        command_buffer,
                        self.image,
                        vk::ImageLayout::GENERAL,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    )
                    .unwrap();

                    let region = vk::BufferImageCopy::default()
                        .buffer_image_height(self.extent.height)
                        .buffer_row_length(self.extent.width)
                        .buffer_offset(0)
                        .image_extent(self.extent.into())
                        .image_offset(vk::Offset3D::default())
                        .image_subresource(
                            vk::ImageSubresourceLayers::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .base_array_layer(0)
                                .layer_count(1)
                                .mip_level(0),
                        );

                    vk.device().cmd_copy_image_to_buffer(
                        command_buffer,
                        self.image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        staging_buffer,
                        slice::from_ref(&region),
                    );
                },
                "Copy to Staging",
            )?;
        }

        // Copy tonemapped staging to cpu
        let tonemapped_bytes = unsafe {
            let pointer = vulkan
                .device()
                .map_memory(
                    staging_memory,
                    0,
                    self.extent.width as u64 * self.extent.height as u64 * 4,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(|e| VkError::new(e, "vkMapMemory"))?;

            let bytes: Vec<u8> = slice::from_raw_parts(
                pointer as _,
                self.extent.width as usize * self.extent.height as usize * 4,
            )
            .to_vec();

            vulkan.device().unmap_memory(staging_memory);

            bytes
        };

        // Free resources
        {
            vulkan.device().destroy_buffer(staging_buffer, None);
            vulkan.device().free_memory(staging_memory, None);
        }

        Ok(tonemapped_bytes)
    }

    /// Destroy the image.
    pub unsafe fn destroy(self, vulkan: &Vulkan) {
        vulkan.device().destroy_image(self.image, None);
        vulkan.device().free_memory(self.memory, None);
    }
}

/// SDR Image error variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An allocation failed.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
