extern crate alloc;

use alloc::sync::Arc;
use core::slice;
use std::{
    fs::{create_dir_all, File},
    io::Write,
};

use ash::vk;
use ash_helper::{allocate_buffer, cmd_transition_image, onetime_command, VulkanContext};
use flate2::{write::ZlibEncoder, Compression};
use testing::{setup_logger, Metadata};
use vulkan::{HdrImage, QueuePurpose, Vulkan};
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

fn main() {
    let _logger = setup_logger().unwrap();

    let vulkan = Arc::new(unsafe { Vulkan::new(true, None) }.unwrap());

    let (monitor, capture) = {
        let dx = DirectX::new().unwrap();
        let mut cache = CaptureItemCache::new();

        // Get capture
        let monitor = Monitor::get_hovered_monitor(&dx).unwrap().unwrap();
        let capture_item = { cache.get_capture_item(monitor.handle.0).unwrap() };
        let (capture, resources) = { WindowsCapture::take_capture(&dx, &capture_item).unwrap() };

        resources.destroy(&dx).unwrap();

        (monitor, capture)
    };

    let hdr_capture = unsafe {
        HdrImage::import_windows_capture(&vulkan, capture.size, capture.handle.0 .0 as isize)
            .unwrap()
    };

    // transition image layout
    unsafe {
        onetime_command(
            vulkan.as_ref(),
            vulkan.transient_pool(),
            vulkan.queue(QueuePurpose::Compute),
            |vk, command_buffer| {
                cmd_transition_image(
                    vk,
                    command_buffer,
                    hdr_capture.image,
                    vk::ImageLayout::GENERAL,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                )
                .unwrap()
            },
            "Transition capture",
        )
        .unwrap();
    }

    // Allocate staging buffer
    let buffer_size = capture.size[0] as u64 * capture.size[1] as u64 * 4 * 2;
    let (staging_buffer, staging_memory, _) = unsafe {
        let queue_family = vulkan.queue_family_index();
        let create_info = vk::BufferCreateInfo::default()
            .queue_family_indices(slice::from_ref(&queue_family))
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST);

        allocate_buffer(
            vulkan.as_ref(),
            &create_info,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            "Staging",
        )
        .unwrap()
    };

    // Copy to staging
    unsafe {
        onetime_command(
            vulkan.as_ref(),
            vulkan.transient_pool(),
            vulkan.queue(QueuePurpose::Compute),
            |vk, command_buffer| {
                let regions = vk::BufferImageCopy::default()
                    .buffer_image_height(hdr_capture.extent.height)
                    .buffer_offset(0)
                    .buffer_row_length(hdr_capture.extent.width)
                    .image_extent(hdr_capture.extent.into())
                    .image_offset(vk::Offset3D::default())
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0),
                    );

                vk.device().cmd_copy_image_to_buffer(
                    command_buffer,
                    hdr_capture.image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    staging_buffer,
                    slice::from_ref(&regions),
                );
            },
            "Copy capture",
        )
        .unwrap();
    }

    // Copy to CPU
    let bytes = unsafe {
        let pointer = vulkan
            .device()
            .map_memory(
                staging_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();

        let slice: &[u8] = slice::from_raw_parts(pointer as _, buffer_size as usize);

        vulkan.device().unmap_memory(staging_memory);

        slice
    };

    // Save Metadata to file
    let metadata = Metadata {
        width: capture.size[0],
        height: capture.size[1],
        sdr_white: monitor.sdr_white,
    };
    let toml = toml::to_string_pretty(&metadata).unwrap();
    let mut file = File::create("crates/testing/src/assets/metadata.toml").unwrap();
    file.write_all(toml.as_bytes()).unwrap();

    // Save bytes to file
    create_dir_all("crates/testing/src/assets").unwrap();
    let file = File::create("crates/testing/src/assets/hdr-capture.raw").unwrap();
    let mut file = ZlibEncoder::new(file, Compression::best());
    file.write_all(bytes).unwrap();
    file.flush().unwrap();

    unsafe {
        hdr_capture.destroy(&vulkan);

        vulkan.device().destroy_buffer(staging_buffer, None);
        vulkan.device().free_memory(staging_memory, None);
    }
}
