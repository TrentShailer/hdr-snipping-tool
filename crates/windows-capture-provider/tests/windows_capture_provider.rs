use std::sync::mpsc::channel;

use ash::vk::{
    AccessFlags2, DependencyInfo, DeviceMemory, Extent2D, ExternalMemoryHandleTypeFlags,
    ExternalMemoryImageCreateInfo, Format, Image, ImageAspectFlags, ImageCreateInfo, ImageLayout,
    ImageMemoryBarrier2, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags,
    ImageViewCreateInfo, ImageViewType, ImportMemoryWin32HandleInfoKHR, MemoryAllocateInfo,
    MemoryDedicatedAllocateInfo, MemoryPropertyFlags, PipelineStageFlags2, SampleCountFlags,
    SharingMode, QUEUE_FAMILY_IGNORED,
};
use scrgb_tonemapper::tonemap;
use test_helper::{get_window::get_window, logger::init_logger};
use vulkan_instance::{CommandBufferUsage, VulkanInstance};
use windows::Win32::Foundation::CloseHandle;
use windows_capture_provider::{get_capture::get_capture, Capture, DirectXDevices, DisplayCache};

#[test]
fn windows_capture_provider() {
    init_logger();

    let (close_sender, close_receiver) = channel();
    get_window(
        |window| {
            let vk = VulkanInstance::new(window, true).unwrap();

            inner(&vk);

            close_sender.send(()).unwrap()
        },
        close_receiver,
    );
}

pub fn inner(vk: &VulkanInstance) {
    let dx = DirectXDevices::new().unwrap();
    let mut display_cache = DisplayCache::new(&dx).unwrap();

    display_cache.refresh(&dx).unwrap();

    let display = display_cache.hovered().unwrap().unwrap();

    let capture_item = display_cache
        .capture_items
        .get(&(display.handle.0 as isize))
        .unwrap();

    let capture = get_capture(&dx, &display, capture_item).unwrap();

    let (image, memory) = import_capture(vk, &capture);

    save_image(vk, image, &capture);

    unsafe {
        vk.device.destroy_image(image, None);
        vk.device.free_memory(memory, None);

        CloseHandle(capture.handle).unwrap();
    }
}

pub fn save_image(vk: &VulkanInstance, image: Image, capture: &Capture) {
    let image_view = unsafe {
        let image_view_create_info = ImageViewCreateInfo {
            image,
            view_type: ImageViewType::TYPE_2D,
            format: Format::R16G16B16A16_SFLOAT,
            subresource_range: ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        vk.device
            .create_image_view(&image_view_create_info, None)
            .unwrap()
    };

    let srgb = tonemap(vk, image_view, capture.display.size, 1.0).unwrap();

    let raw_capture = srgb.copy_to_box(vk).unwrap();

    test_helper::save_image::save_image(
        "windows-capture-provider",
        raw_capture,
        capture.display.size,
    );

    unsafe { vk.device.destroy_image_view(image_view, None) };
}

pub fn import_capture(vk: &VulkanInstance, capture: &Capture) -> (Image, DeviceMemory) {
    let image = unsafe {
        let image_extent = Extent2D {
            width: capture.display.size[0],
            height: capture.display.size[1],
        };

        let mut external_memory_image = ExternalMemoryImageCreateInfo::default()
            .handle_types(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

        let image_create_info = ImageCreateInfo {
            image_type: ImageType::TYPE_2D,
            format: Format::R16G16B16A16_SFLOAT,
            extent: image_extent.into(),
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCountFlags::TYPE_1,
            tiling: ImageTiling::OPTIMAL,
            usage: ImageUsageFlags::STORAGE,
            sharing_mode: SharingMode::EXCLUSIVE,
            initial_layout: ImageLayout::UNDEFINED,
            ..Default::default()
        }
        .push_next(&mut external_memory_image);

        vk.device.create_image(&image_create_info, None).unwrap()
    };

    let memory = unsafe {
        let memory_requirement = vk.device.get_image_memory_requirements(image);

        let memory_index = vk
            .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
            .unwrap();

        let mut dedicated_allocation = MemoryDedicatedAllocateInfo::default().image(image);
        let mut import_info = ImportMemoryWin32HandleInfoKHR::default()
            .handle_type(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
            .handle(capture.handle.0 as isize);

        let allocate_info = MemoryAllocateInfo::default()
            .allocation_size(memory_requirement.size)
            .memory_type_index(memory_index)
            .push_next(&mut import_info)
            .push_next(&mut dedicated_allocation);

        let device_memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

        vk.device
            .bind_image_memory(image, device_memory, 0)
            .unwrap();

        device_memory
    };

    // transition image layout
    vk.record_submit_command_buffer(
        CommandBufferUsage::Setup,
        &[],
        &[],
        |device, command_buffer| {
            let memory_barriers = [ImageMemoryBarrier2 {
                src_stage_mask: PipelineStageFlags2::NONE,
                src_access_mask: AccessFlags2::NONE,
                dst_stage_mask: PipelineStageFlags2::NONE,
                dst_access_mask: AccessFlags2::NONE,
                old_layout: ImageLayout::UNDEFINED,
                new_layout: ImageLayout::GENERAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            }];

            let dependency_info = DependencyInfo::default().image_memory_barriers(&memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }
        },
    )
    .unwrap();

    (image, memory)
}
