//! General tests for the Vulkan Context
//!

use ash::vk;
use ash_helper::{VK_GLOBAL_ALLOCATOR, VulkanContext, allocate_buffer};

use vulkan::Vulkan;

#[test]
fn valid_vulkan_context() {
    let vulkan = Vulkan::new(
        true,
        std::env::current_exe().unwrap().parent().unwrap(),
        None,
    )
    .unwrap();

    let create_info = vk::BufferCreateInfo::default()
        .size(1024 * 1024 * 4)
        .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
        .queue_family_indices(vulkan.queue_family_index_as_slice());

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
        vulkan
            .device()
            .free_memory(memory, VK_GLOBAL_ALLOCATOR.as_deref());
        vulkan
            .device()
            .destroy_buffer(buffer, VK_GLOBAL_ALLOCATOR.as_deref());
    }

    drop(vulkan);
}
