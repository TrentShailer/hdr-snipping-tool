extern crate alloc;

use alloc::sync::Arc;
use core::slice;

use ash::vk;
use ash_helper::{allocate_buffer, cmd_transition_image, onetime_command, VulkanContext};
use image::ColorType;
use testing::{load_hdr_capture, setup_logger};
use tracing::{info, info_span};
use vulkan::{HdrToSdrTonemapper, QueuePurpose, Vulkan};

fn main() {
    let _guards = setup_logger().unwrap();

    let vulkan = unsafe { Arc::new(Vulkan::new(true, None).unwrap()) };
    let tonemapper = unsafe { HdrToSdrTonemapper::new(vulkan.clone()) }.unwrap();

    let (metadata, hdr_image) = load_hdr_capture(&vulkan);
    let extent = vk::Extent2D::default()
        .width(metadata.width)
        .height(metadata.height);

    let sdr_image = unsafe { tonemapper.tonemap(hdr_image, metadata.sdr_white) }.unwrap();
    info!("Tonemapped Image");

    // Create staging
    let (staging_buffer, staging_memory) = {
        let queue_family = vulkan.queue_family_index();

        let buffer_info = vk::BufferCreateInfo::default()
            .queue_family_indices(slice::from_ref(&queue_family))
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .size(extent.width as u64 * extent.height as u64 * 4);

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
                    .buffer_image_height(metadata.height)
                    .buffer_row_length(metadata.width)
                    .buffer_offset(0)
                    .image_extent(extent.into())
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
            .map_memory(
                staging_memory,
                0,
                metadata.width as u64 * metadata.height as u64 * 4,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();

        let slice: &[u8] = slice::from_raw_parts(
            pointer as _,
            metadata.width as usize * metadata.height as usize * 4,
        );

        vulkan.device().unmap_memory(staging_memory);

        slice
    };
    {
        let _span = info_span!("Save Image").entered();
        image::save_buffer_with_format(
            "crates/testing/src/assets/tonemapped.png",
            tonemapped_bytes,
            metadata.width,
            metadata.height,
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
