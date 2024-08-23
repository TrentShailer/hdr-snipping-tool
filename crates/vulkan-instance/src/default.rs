use ash::vk::{
    BufferMemoryBarrier2, ImageAspectFlags, ImageMemoryBarrier2, ImageSubresourceRange,
    QUEUE_FAMILY_IGNORED,
};

use crate::VulkanInstance;

impl VulkanInstance {
    /// Provides better defaults for BufferMemoryBarrier2
    pub fn buffer_memory_barrier<'a>() -> BufferMemoryBarrier2<'a> {
        BufferMemoryBarrier2::default()
            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
    }

    /// Provides better defaults for ImageMemoryBarrier2
    pub fn image_memory_barrier<'a>() -> ImageMemoryBarrier2<'a> {
        let subresource_range = ImageSubresourceRange::default()
            .layer_count(1)
            .level_count(1)
            .aspect_mask(ImageAspectFlags::COLOR);

        ImageMemoryBarrier2::default()
            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
            .subresource_range(subresource_range)
    }
}
