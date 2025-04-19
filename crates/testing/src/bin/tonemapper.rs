extern crate alloc;

use alloc::sync::Arc;
use core::slice;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

use ash::vk;
use ash_helper::{VulkanContext, allocate_buffer, cmd_transition_image, onetime_command};
use image::ColorType;
use testing::setup_logger;
use tracing::info_span;
use vulkan::{HdrImage, HdrScanner, HdrToSdrTonemapper, QueuePurpose, Vulkan};

fn main() {
    let _guards = setup_logger().unwrap();

    let vulkan = Arc::new(
        Vulkan::new(
            true,
            std::env::current_exe().unwrap().parent().unwrap(),
            None,
        )
        .unwrap(),
    );
    let tonemapper = HdrToSdrTonemapper::new(vulkan.clone()).unwrap();

    let (hdr_image, whitepoint) = {
        let direct_x = DirectX::new().unwrap();

        let monitor = Monitor::get_hovered_monitor(&direct_x)
            .unwrap()
            .expect("Monitor should be some");

        let mut cache = CaptureItemCache::new();
        let capture_item = cache.get_capture_item(monitor.handle.0).unwrap();

        let (capture, resources) = WindowsCapture::take_capture(&direct_x, &capture_item).unwrap();

        let hdr_image = unsafe {
            HdrImage::import_windows_capture(&vulkan, capture.size, capture.handle.0.0 as isize)
                .unwrap()
        };

        let mut hdr_scanner = HdrScanner::new(Arc::clone(&vulkan)).unwrap();

        let maximum = unsafe {
            let _span = info_span!("HDR Scanner").entered();
            hdr_scanner.scan(hdr_image).unwrap()
        };

        let whitepoint = if maximum <= monitor.sdr_white {
            monitor.sdr_white
        } else {
            monitor.max_brightness
        };

        unsafe { resources.destroy(&direct_x).unwrap() };

        (hdr_image, whitepoint)
    };

    let image_values = hdr_image.extent.width as u64 * hdr_image.extent.height as u64 * 4;

    let sdr_image = unsafe {
        let _span = info_span!("Tonemap").entered();
        tonemapper.tonemap(hdr_image, whitepoint).unwrap()
    };

    // Create staging
    let (staging_buffer, staging_memory) = {
        let buffer_info = vk::BufferCreateInfo::default()
            .queue_family_indices(vulkan.queue_family_index_as_slice())
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .size(image_values);

        let (buffer, memory, _) = unsafe {
            allocate_buffer(
                vulkan.as_ref(),
                &buffer_info,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                "Staging",
            )
            .unwrap()
        };

        (buffer, memory)
    };

    // Copy tonemapped image to staging
    unsafe {
        let _span = info_span!("Copy GPU to Staging").entered();

        onetime_command(
            vulkan.as_ref(),
            vulkan.transient_pool(),
            vulkan.queue(QueuePurpose::Compute),
            |vk, command_buffer| {
                cmd_transition_image(
                    vk,
                    command_buffer,
                    sdr_image.image,
                    vk::ImageLayout::GENERAL,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                )
                .unwrap();

                let region = vk::BufferImageCopy::default()
                    .buffer_image_height(hdr_image.extent.height)
                    .buffer_row_length(hdr_image.extent.width)
                    .buffer_offset(0)
                    .image_extent(hdr_image.extent.into())
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
                    sdr_image.image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    staging_buffer,
                    slice::from_ref(&region),
                );
            },
            "Copy to Staging",
        )
        .unwrap();
    }

    // Copy tonemapped staging to cpu
    let tonemapped_bytes = unsafe {
        let _span = info_span!("Copy staging to CPU").entered();
        let pointer = vulkan
            .device()
            .map_memory(staging_memory, 0, image_values, vk::MemoryMapFlags::empty())
            .unwrap();

        let slice: &[u8] = slice::from_raw_parts(pointer as _, image_values as usize);

        vulkan.device().unmap_memory(staging_memory);

        slice
    };
    {
        let _span = info_span!("Save Image").entered();
        image::save_buffer_with_format(
            "crates/testing/output/tonemapped.png",
            tonemapped_bytes,
            hdr_image.extent.width,
            hdr_image.extent.height,
            ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();
    }

    unsafe {
        vulkan.device().destroy_buffer(staging_buffer, None);
        vulkan.device().free_memory(staging_memory, None);
        hdr_image.destroy(&vulkan);
        sdr_image.destroy(&vulkan);
    }
}
