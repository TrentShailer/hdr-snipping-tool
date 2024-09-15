use ash::{
    vk::{self, CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool, Fence},
    Device,
};
use tracing::instrument;

use crate::VulkanError;

use super::Error;

#[instrument(skip_all, level = tracing::Level::DEBUG, err)]
pub fn get_command_buffer(
    device: &Device,
    queue_family_index: u32,
) -> Result<(CommandPool, (CommandBuffer, Fence)), Error> {
    let command_pool_create_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);

    let command_buffer_pool =
        unsafe { device.create_command_pool(&command_pool_create_info, None) }
            .map_err(|e| VulkanError::VkResult(e, "creating command pool"))?;

    let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
        .command_buffer_count(1)
        .command_pool(command_buffer_pool)
        .level(CommandBufferLevel::PRIMARY);

    let command_buffer = unsafe {
        device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .map_err(|e| VulkanError::VkResult(e, "allocating command buffers"))?[0]
    };

    let fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
    let fence = unsafe { device.create_fence(&fence_create_info, None) }
        .map_err(|e| VulkanError::VkResult(e, "creating fence"))?;

    Ok((command_buffer_pool, (command_buffer, fence)))
}
