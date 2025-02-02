extern crate alloc;

use alloc::sync::Arc;
use core::slice;

use ash::{util::Align, vk};
use ash_helper::{
    allocate_buffer, allocate_image, cmd_transition_image, onetime_command, VulkanContext,
};
use half::f16;
use testing::setup_logger;
use vulkan::{HdrImage, HistogramGenerator, QueuePurpose, Vulkan};

const EXTENT: vk::Extent2D = vk::Extent2D {
    width: 2u32.pow(13),
    height: 2u32.pow(13),
};
const VALUES: u64 = EXTENT.width as u64 * EXTENT.height as u64 * 4;
const MAXIMUM: f32 = 15.5;

fn main() {
    let _logger = setup_logger().unwrap();

    let vulkan = unsafe { Arc::new(Vulkan::new(true, None).unwrap()) };

    let mut histogram = unsafe { HistogramGenerator::new(vulkan.clone()).unwrap() };

    let (image, image_memory) = {
        let queue_family = vulkan.queue_family_index();
        let image_create_info = vk::ImageCreateInfo::default()
            .array_layers(1)
            .extent(EXTENT.into())
            .format(vk::Format::R16G16B16A16_SFLOAT)
            .image_type(vk::ImageType::TYPE_2D)
            .flags(vk::ImageCreateFlags::empty())
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .mip_levels(1)
            .queue_family_indices(slice::from_ref(&queue_family))
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE);

        let (image, memory, _) = unsafe {
            allocate_image(
                vulkan.as_ref(),
                &image_create_info,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                "Image",
            )
            .unwrap()
        };

        (image, memory)
    };

    let (staging_buffer, staging_memory) = {
        let queue_family = vulkan.queue_family_index();

        let buffer_info = vk::BufferCreateInfo::default()
            .queue_family_indices(slice::from_ref(&queue_family))
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .size(VALUES * size_of::<f16>() as u64);

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

    let data = vec![f16::from_f32(5.0); VALUES as usize];

    // Copy data to GPU
    {
        // Copy data to staging
        {
            let pointer = unsafe {
                vulkan
                    .device()
                    .map_memory(
                        staging_memory,
                        0,
                        VALUES * size_of::<f16>() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap()
            };

            let mut align: Align<f16> = unsafe {
                Align::new(
                    pointer,
                    align_of::<f16>() as u64,
                    VALUES * size_of::<f16>() as u64,
                )
            };
            align.copy_from_slice(&data);

            unsafe { vulkan.device().unmap_memory(staging_memory) };
        }

        unsafe {
            onetime_command(
                vulkan.as_ref(),
                vulkan.transient_pool(),
                vulkan.queue(QueuePurpose::Compute),
                |vk, command_buffer| {
                    // Transition to Transfer DST Optimal
                    cmd_transition_image(
                        vk,
                        command_buffer,
                        image,
                        vk::ImageLayout::UNDEFINED,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    )
                    .unwrap();

                    // Copy to image
                    {
                        let subresource = vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .mip_level(0)
                            .layer_count(1);

                        let image_copy = vk::BufferImageCopy::default()
                            .buffer_image_height(EXTENT.height)
                            .buffer_row_length(EXTENT.width)
                            .buffer_offset(0)
                            .image_extent(EXTENT.into())
                            .image_subresource(subresource);

                        vk.device().cmd_copy_buffer_to_image(
                            command_buffer,
                            staging_buffer,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            slice::from_ref(&image_copy),
                        );
                    }

                    // Transition to general
                    cmd_transition_image(
                        vk,
                        command_buffer,
                        image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        vk::ImageLayout::GENERAL,
                    )
                    .unwrap();
                },
                "Copy Data to GPU",
            )
            .unwrap();
        }
    }

    let image_view = {
        let create_info = vk::ImageViewCreateInfo::default()
            .components(vk::ComponentMapping::default())
            .format(vk::Format::R16G16B16A16_SFLOAT)
            .image(image)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .layer_count(1)
                    .level_count(1),
            )
            .view_type(vk::ImageViewType::TYPE_2D);
        unsafe {
            vulkan
                .device()
                .create_image_view(&create_info, None)
                .unwrap()
        }
    };

    let hdr_image = HdrImage {
        image,
        memory: image_memory,
        view: image_view,
        extent: EXTENT,
    };

    let histogram = { unsafe { histogram.generate(hdr_image, MAXIMUM).unwrap() } };

    dbg!(&histogram);

    let total = histogram
        .clone()
        .into_iter()
        .reduce(|acc, v| acc + v)
        .unwrap();
    assert_eq!(total, EXTENT.width * EXTENT.height * 3);

    unsafe {
        vulkan.device().destroy_buffer(staging_buffer, None);
        vulkan.device().free_memory(staging_memory, None);
        hdr_image.destroy(&vulkan);
    }
}
