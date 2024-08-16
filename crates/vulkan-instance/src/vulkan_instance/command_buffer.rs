use std::{collections::HashMap, sync::Arc};

use ash::{
    vk::{self, CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool},
    Device,
};

use crate::CommandBufferUsage;

use super::Error;

pub fn get_command_buffers(
    device: Arc<Device>,
    queue_family_index: u32,
) -> Result<(CommandPool, HashMap<CommandBufferUsage, CommandBuffer>), Error> {
    let command_pool_create_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family_index);

    let command_buffer_pool =
        unsafe { device.create_command_pool(&command_pool_create_info, None) }
            .map_err(|e| Error::Vulkan(e, "creating command pool"))?;

    let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
        .command_buffer_count(CommandBufferUsage::VALUES.len() as u32)
        .command_pool(command_buffer_pool)
        .level(CommandBufferLevel::PRIMARY);

    let command_buffers_vec =
        unsafe { device.allocate_command_buffers(&command_buffer_allocate_info) }
            .map_err(|e| Error::Vulkan(e, "allocating command buffers"))?;

    let mut command_buffers = HashMap::new();
    for (index, value) in CommandBufferUsage::VALUES.into_iter().enumerate() {
        command_buffers.insert(value, command_buffers_vec[index]);
    }

    Ok((command_buffer_pool, command_buffers))
}
