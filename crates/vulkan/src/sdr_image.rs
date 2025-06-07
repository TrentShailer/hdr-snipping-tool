use core::slice;

use ash::vk;
use ash_helper::{
    AllocationError, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext, allocate_buffer,
    cmd_transition_image, onetime_command,
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
    pub unsafe fn copy_to_cpu(
        &self,
        vulkan: &Vulkan,
        selection_position: [usize; 2],
        selection_size: [usize; 2],
    ) -> Result<Vec<u8>, SdrImageError> {
        let selection_x = selection_position[0];
        let selection_y = selection_position[1];
        let selection_width = selection_size[0];
        let selection_height = selection_size[1];

        // Create staging
        let (staging_buffer, staging_memory) = {
            let buffer_info = vk::BufferCreateInfo::default()
                .queue_family_indices(vulkan.queue_family_index_as_slice())
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .size(u64::from(self.extent.width) * u64::from(self.extent.height) * 4);

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
                    #[allow(clippy::missing_panics_doc)]
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

        let mut bytes: Vec<u8> = vec![0; selection_size[0] * selection_size[1] * 4];

        // Copy tone-mapped staging to CPU
        unsafe {
            let pointer = vulkan
                .device()
                .map_memory(
                    staging_memory,
                    0,
                    u64::from(self.extent.width) * u64::from(self.extent.height) * 4,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(|e| VkError::new(e, "vkMapMemory"))?;

            let raw = slice::from_raw_parts(
                pointer as _,
                self.extent.width as usize * self.extent.height as usize * 4,
            );
            let raw_start = selection_y * self.extent.width as usize * 4 + selection_x * 4;

            for row in 0..selection_height {
                let output_start = row * selection_width * 4;
                let output_end = output_start + selection_width * 4;

                let input_start = raw_start + row * self.extent.width as usize * 4;
                let input_end = input_start + selection_width * 4;

                bytes[output_start..output_end].copy_from_slice(&raw[input_start..input_end]);
            }

            vulkan.device().unmap_memory(staging_memory);
        };

        // Free resources
        unsafe {
            vulkan
                .device()
                .destroy_buffer(staging_buffer, VK_GLOBAL_ALLOCATOR.as_deref());
            vulkan
                .device()
                .free_memory(staging_memory, VK_GLOBAL_ALLOCATOR.as_deref());
        }

        Ok(bytes)
    }

    /// Destroy the image.
    pub unsafe fn destroy(self, vulkan: &Vulkan) {
        unsafe {
            vulkan
                .device()
                .destroy_image(self.image, VK_GLOBAL_ALLOCATOR.as_deref());
            vulkan
                .device()
                .free_memory(self.memory, VK_GLOBAL_ALLOCATOR.as_deref());
        }
    }
}

/// SDR Image error variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SdrImageError {
    /// An allocation failed.
    #[error(transparent)]
    AllocationError(#[from] AllocationError),

    /// A Vulkan call returned an error.
    #[error(transparent)]
    VkError(#[from] VkError),
}
