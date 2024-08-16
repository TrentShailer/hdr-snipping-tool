use std::sync::mpsc::channel;

use ash::{
    util::Align,
    vk::{
        BufferCreateInfo, BufferUsageFlags, MemoryAllocateInfo, MemoryMapFlags,
        MemoryPropertyFlags, SharingMode,
    },
};
use test_helper::{get_window::get_window, logger::init_logger};
use vulkan_instance::VulkanInstance;

#[test]
fn staging_buffer() {
    init_logger();

    let (close_sender, close_receiver) = channel();
    get_window(
        |window| {
            let vk = VulkanInstance::new(window, true).unwrap();

            unsafe {
                let buffer_create_info = BufferCreateInfo {
                    size: 1024,
                    usage: BufferUsageFlags::TRANSFER_SRC,
                    sharing_mode: SharingMode::EXCLUSIVE,
                    ..Default::default()
                };

                let buffer = vk.device.create_buffer(&buffer_create_info, None).unwrap();

                let memory_requirements = vk.device.get_buffer_memory_requirements(buffer);

                let memory_index = vk
                    .find_memorytype_index(
                        &memory_requirements,
                        MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
                    )
                    .unwrap();

                let allocate_info = MemoryAllocateInfo {
                    allocation_size: memory_requirements.size,
                    memory_type_index: memory_index,
                    ..Default::default()
                };

                let memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

                vk.device.bind_buffer_memory(buffer, memory, 0).unwrap();

                let staging_ptr = vk
                    .device
                    .map_memory(memory, 0, 256, MemoryMapFlags::empty())
                    .unwrap();
                let data: Box<[u32]> = (0..=255u32).collect();

                let mut staging_slice =
                    Align::new(staging_ptr, std::mem::align_of::<u32>() as u64, 256);
                staging_slice.copy_from_slice(&data);
                vk.device.unmap_memory(memory);

                // verify

                let staging_ptr = vk
                    .device
                    .map_memory(memory, 0, 256, MemoryMapFlags::empty())
                    .unwrap();

                let slice_data: &[u32] = std::slice::from_raw_parts(staging_ptr.cast(), 256);
                let data_slice: &[u32] = &data;
                assert_eq!(slice_data, data_slice);

                vk.device.unmap_memory(memory);
                vk.device.destroy_buffer(buffer, None);
                vk.device.free_memory(memory, None);
            };

            close_sender.send(()).unwrap()
        },
        close_receiver,
    );
}
