use alloc::sync::Arc;
use core::slice;

use ash::vk;
use ash_helper::{allocate_buffer, try_name, VkError, VulkanContext};
use half::f16;

use crate::vulkan::Vulkan;

use super::{
    buffer_scanner::BufferScanner, image_scanner::ImageScanner, Error, HdrScanner, COMMAND_BUFFERS,
};

impl HdrScanner {
    /// Create a new instance of an HdrScanner.
    pub unsafe fn new(vulkan: Arc<Vulkan>) -> Result<Self, Error> {
        // Create executor command pools and buffers
        let command_objects: Vec<_> = {
            (0..COMMAND_BUFFERS)
                .map::<Result<_, Error>, _>(|index| {
                    let pool = {
                        let pool_create_info = vk::CommandPoolCreateInfo::default()
                            .queue_family_index(vulkan.queue_family_index());

                        unsafe { vulkan.device().create_command_pool(&pool_create_info, None) }
                            .map_err(|e| VkError::new(e, "vkCreateCommandPool"))?
                    };

                    let buffer = {
                        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                            .command_pool(pool)
                            .level(vk::CommandBufferLevel::PRIMARY)
                            .command_buffer_count(1);

                        unsafe {
                            vulkan
                                .device()
                                .allocate_command_buffers(&command_buffer_info)
                        }
                        .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0]
                    };

                    // Debug: Name the objects.
                    unsafe {
                        try_name(vulkan.as_ref(), pool, &format!("HdrScanner Pool {index}"));
                        try_name(
                            vulkan.as_ref(),
                            buffer,
                            &format!("HdrScanner Buffer {index}"),
                        );
                    }

                    Ok((pool, buffer))
                })
                .collect::<Result<_, _>>()?
        };

        // Create timeline semaphore
        let semaphore = {
            let mut type_info = vk::SemaphoreTypeCreateInfo::default()
                .initial_value(0)
                .semaphore_type(vk::SemaphoreType::TIMELINE);
            let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut type_info);

            let semaphore = unsafe { vulkan.device().create_semaphore(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateSemaphore"))?;

            try_name(vulkan.as_ref(), semaphore, "HdrScanner Semaphore");

            semaphore
        };

        let image_scanner = unsafe { ImageScanner::new(&vulkan)? };
        let buffer_scanner = unsafe { BufferScanner::new(&vulkan)? };

        // Host memory for copying the result
        let (host_buffer, host_memory, _) = {
            let queue_family = vulkan.queue_family_index();

            let create_info = vk::BufferCreateInfo::default()
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .size(4 * size_of::<f16>() as u64)
                .queue_family_indices(slice::from_ref(&queue_family));

            unsafe {
                allocate_buffer(
                    vulkan.as_ref(),
                    &create_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "HdrScanner Staging",
                )?
            }
        };

        Ok(Self {
            vulkan,

            command_objects,
            semaphore,
            semaphore_value: 0,

            host_memory,
            host_buffer,

            buffer_scanner,
            image_scanner,

            resources: None,
        })
    }
}
