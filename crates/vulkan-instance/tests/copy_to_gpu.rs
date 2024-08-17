use std::{
    sync::{mpsc::channel, Arc},
    u64,
};

use ash::{
    util::Align,
    vk::{
        AccessFlags2, BufferCopy2, BufferCreateInfo, BufferMemoryBarrier2, BufferUsageFlags,
        CopyBufferInfo2, DependencyInfo, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags,
        PipelineStageFlags2, SharingMode, QUEUE_FAMILY_IGNORED,
    },
};

use test_helper::{get_window::get_window, logger::init_logger};
use vulkan_instance::{CommandBufferUsage, VulkanInstance};
use winit::window::Window;

#[test]
fn copy_to_gpu() {
    init_logger();

    let (close_sender, close_receiver) = channel();
    get_window(
        |window| {
            copy_to_gpu_inner(window);

            close_sender.send(()).unwrap()
        },
        close_receiver,
    );
}

fn copy_to_gpu_inner(window: Arc<Window>) {
    let vk = VulkanInstance::new(window, true).unwrap();

    // create and write to staging buffer
    let (staging_buffer, staging_buffer_memory) = unsafe {
        let buffer_create_info = BufferCreateInfo::default()
            .size(1024)
            .usage(BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let staging_buffer = vk.device.create_buffer(&buffer_create_info, None).unwrap();

        let staging_buffer_memory_requirements =
            vk.device.get_buffer_memory_requirements(staging_buffer);

        let staging_buffer_memory_index = vk
            .find_memorytype_index(
                &staging_buffer_memory_requirements,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )
            .unwrap();

        let staging_buffer_allocate_info = MemoryAllocateInfo::default()
            .allocation_size(staging_buffer_memory_requirements.size)
            .memory_type_index(staging_buffer_memory_index);

        let staging_buffer_memory = vk
            .device
            .allocate_memory(&staging_buffer_allocate_info, None)
            .unwrap();

        let staging_ptr = vk
            .device
            .map_memory(staging_buffer_memory, 0, 1024, MemoryMapFlags::empty())
            .unwrap();
        let data: Box<[u32]> = (0..1024u32).collect();

        let mut staging_slice = Align::new(staging_ptr, std::mem::align_of::<u32>() as u64, 1024);
        staging_slice.copy_from_slice(&data);
        vk.device.unmap_memory(staging_buffer_memory);

        vk.device
            .bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)
            .unwrap();

        (staging_buffer, staging_buffer_memory)
    };

    // create gpu buffer
    let (gpu_buffer, gpu_buffer_memory) = unsafe {
        let buffer_create_info = BufferCreateInfo::default()
            .size(1024)
            .usage(BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(SharingMode::EXCLUSIVE);

        let buffer = vk.device.create_buffer(&buffer_create_info, None).unwrap();

        let memory_requirements = vk.device.get_buffer_memory_requirements(buffer);

        let memory_index = vk
            .find_memorytype_index(&memory_requirements, MemoryPropertyFlags::DEVICE_LOCAL)
            .unwrap();

        let allocate_info = MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_index);

        let memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

        vk.device.bind_buffer_memory(buffer, memory, 0).unwrap();

        (buffer, memory)
    };

    // copy from staging to gpu
    vk.record_submit_command_buffer(
        CommandBufferUsage::Setup,
        &[],
        &[],
        |device, command_buffer| {
            let memory_barrier = BufferMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::NONE,
                src_access_mask: AccessFlags2::NONE,
                dst_stage_mask: PipelineStageFlags2::TRANSFER,
                dst_access_mask: AccessFlags2::MEMORY_WRITE,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                buffer: gpu_buffer,
                offset: 0,
                size: 1024,
                ..Default::default()
            };
            let memory_barriers = &[memory_barrier];

            let dependency_info = DependencyInfo::default().buffer_memory_barriers(memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

            // copy
            let buffer_copy = BufferCopy2 {
                src_offset: 0,
                dst_offset: 0,
                size: 1024,
                ..Default::default()
            };
            let buffer_copy_regions = &[buffer_copy];

            let buffer_copy_info = CopyBufferInfo2::default()
                .src_buffer(staging_buffer)
                .dst_buffer(gpu_buffer)
                .regions(buffer_copy_regions);

            unsafe { device.cmd_copy_buffer2(command_buffer, &buffer_copy_info) }
        },
    )
    .unwrap();

    unsafe {
        vk.device.wait_for_fences(
            &[*vk.fences.get(&CommandBufferUsage::Setup).unwrap()],
            true,
            u64::MAX,
        )
    }
    .unwrap();

    // clean up
    unsafe {
        vk.device.destroy_buffer(staging_buffer, None);
        vk.device.free_memory(staging_buffer_memory, None);

        vk.device.destroy_buffer(gpu_buffer, None);
        vk.device.free_memory(gpu_buffer_memory, None);
    }
}
