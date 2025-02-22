//! General tests for the Vulkan Context
//!

use core::slice;

use ash::vk;
use ash_helper::{VulkanContext, allocate_buffer};

use vulkan::Vulkan;

#[test]
fn valid_vulkan_context() {
    let vulkan = Vulkan::new(true, None).unwrap();

    let create_info = vk::BufferCreateInfo::default()
        .size(1024 * 1024 * 4)
        .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
        .queue_family_indices(slice::from_ref(vulkan.queue_family_index_as_ref()));

    let (buffer, memory, _requirements) = unsafe {
        allocate_buffer(
            &vulkan,
            &create_info,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            "valid_vulkan_context",
        )
        .unwrap()
    };

    unsafe {
        vulkan.device().free_memory(memory, None);
        vulkan.device().destroy_buffer(buffer, None);
    }

    drop(vulkan);
}
