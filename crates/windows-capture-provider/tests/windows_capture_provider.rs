use std::sync::mpsc::channel;

use ash::{
    khr::external_memory_win32,
    vk::{
        DeviceMemory, ExportMemoryAllocateInfo, Extent2D, ExternalMemoryHandleTypeFlags,
        ExternalMemoryImageCreateInfo, Format, Image, ImageCreateInfo, ImageLayout, ImageTiling,
        ImageType, ImageUsageFlags, MemoryAllocateInfo, MemoryDedicatedAllocateInfo,
        MemoryGetWin32HandleInfoKHR, MemoryPropertyFlags, SampleCountFlags, SharingMode,
    },
};
use test_helper::{get_window::get_window, logger::init_logger};
use vulkan_instance::VulkanInstance;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows_capture_provider::{get_capture::get_capture, DirectXDevices, DisplayCache};

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

    let (image, image_memory, handle) = create_capture_image(vk, display.size);

    // unsafe {
    //     vk.device.bind_image_memory(image, image_memory, 0).unwrap();
    // }

    let _capture = get_capture(&dx, &display, capture_item, handle).unwrap();

    unsafe {
        vk.device.destroy_image(image, None);
        vk.device.free_memory(image_memory, None);
        CloseHandle(HANDLE(handle as *mut _)).unwrap();
    }
}

pub fn create_capture_image(
    vk: &VulkanInstance,
    image_size: [u32; 2],
) -> (Image, DeviceMemory, ash::vk::HANDLE) {
    let image = unsafe {
        let image_extent = Extent2D {
            width: image_size[0],
            height: image_size[1],
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

    let image_memory = unsafe {
        let memory_requirement = vk.device.get_image_memory_requirements(image);

        let memory_index = vk
            .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
            .unwrap();

        let mut export_allocate_info = ExportMemoryAllocateInfo::default()
            .handle_types(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

        let mut dedicated_allocation = MemoryDedicatedAllocateInfo {
            image,
            ..Default::default()
        };

        let allocate_info = MemoryAllocateInfo::default()
            .allocation_size(memory_requirement.size)
            .memory_type_index(memory_index)
            .push_next(&mut export_allocate_info)
            .push_next(&mut dedicated_allocation);

        let device_memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

        vk.device
            .bind_image_memory(image, device_memory, 0)
            .unwrap();

        device_memory
    };

    let handle = unsafe {
        let external_mem_device = external_memory_win32::Device::new(&vk.instance, &vk.device);
        let memory_handle_create_info = MemoryGetWin32HandleInfoKHR {
            memory: image_memory,
            handle_type: ExternalMemoryHandleTypeFlags::OPAQUE_WIN32,
            ..Default::default()
        };
        let handle = external_mem_device
            .get_memory_win32_handle(&memory_handle_create_info)
            .unwrap();

        dbg!(handle);

        handle
    };

    (image, image_memory, handle)
}
