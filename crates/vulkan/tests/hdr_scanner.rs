//! Tests for HdrScanner
//!

extern crate alloc;

use core::slice;

use alloc::sync::Arc;

use ash::{util::Align, vk};
use ash_helper::{
    VulkanContext, allocate_buffer, allocate_image, cmd_transition_image, onetime_command,
};
use half::f16;
use rand::Rng;
use rand_distr::Distribution;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use vulkan::{HdrImage, HdrScanner, Vulkan};

const EXTENT: vk::Extent2D = vk::Extent2D {
    width: 4096,
    height: 4096,
};

const VALUES: u64 = EXTENT.width as u64 * EXTENT.height as u64 * 4;

const MAXIMUM_GENERATED: f32 = 12.5;
const MAXIMUM_VALUE: f16 = f16::from_f32_const(15.234);

#[test]
fn hdr_scanner_random_data() {
    let vulkan = Arc::new(
        Vulkan::new(
            true,
            std::env::current_exe().unwrap().parent().unwrap(),
            None,
        )
        .unwrap(),
    );
    let mut hdr_scanner = HdrScanner::new(Arc::clone(&vulkan)).unwrap();

    let (staging_buffer, staging_memory, _) = {
        let create_info = vk::BufferCreateInfo::default()
            .queue_family_indices(vulkan.queue_family_index_as_slice())
            .size(VALUES * 2)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC);

        unsafe {
            allocate_buffer(
                vulkan.as_ref(),
                &create_info,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                "Staging",
            )
            .unwrap()
        }
    };

    let (image, memory, _) = {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R16G16B16A16_SFLOAT)
            .extent(EXTENT.into())
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(
                vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_DST,
            )
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        unsafe {
            allocate_image(
                vulkan.as_ref(),
                &create_info,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                "HDR",
            )
            .unwrap()
        }
    };

    let data = {
        let column = rand::rng().random_range(0..EXTENT.width as usize);
        let row = rand::rng().random_range(0..EXTENT.height as usize);
        let channel = rand::rng().random_range(0..3);

        let max_index = (row * EXTENT.width as usize * 4) + (column * 4) + channel;

        let distribution = rand_distr::Uniform::new(0.0, MAXIMUM_GENERATED).unwrap();
        let data: Vec<_> = (0..VALUES as usize)
            .into_par_iter()
            .map_init(rand::rng, |rng, index| {
                if index == max_index {
                    MAXIMUM_VALUE
                } else {
                    f16::from_f32(distribution.sample(rng))
                }
            })
            .collect();

        data
    };

    // Copy data to staging
    unsafe {
        let pointer = vulkan
            .device()
            .map_memory(
                staging_memory,
                0,
                u64::from(EXTENT.width) * u64::from(EXTENT.height) * 4 * 2,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();

        let mut align: Align<f16> = Align::new(
            pointer,
            align_of::<f16>() as u64,
            u64::from(EXTENT.width) * u64::from(EXTENT.height) * 4 * 2,
        );

        align.copy_from_slice(&data);

        vulkan.device().unmap_memory(staging_memory);
    }

    // Copy data to GPU
    unsafe {
        onetime_command(
            vulkan.as_ref(),
            vulkan.transient_pool(),
            vulkan.queue(vulkan::QueuePurpose::Compute),
            |vulkan, command_buffer| {
                cmd_transition_image(
                    vulkan,
                    command_buffer,
                    image,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                )
                .unwrap();

                let region = vk::BufferImageCopy::default()
                    .buffer_image_height(EXTENT.height)
                    .buffer_row_length(EXTENT.width)
                    .buffer_offset(0)
                    .image_extent(EXTENT.into())
                    .image_offset(vk::Offset3D::default())
                    .image_subresource(
                        vk::ImageSubresourceLayers::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(1)
                            .mip_level(0),
                    );

                vulkan.device().cmd_copy_buffer_to_image(
                    command_buffer,
                    staging_buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    slice::from_ref(&region),
                );

                cmd_transition_image(
                    vulkan,
                    command_buffer,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::GENERAL,
                )
                .unwrap();
            },
            "Copy to GPU",
        )
        .unwrap();

        vulkan.device().destroy_buffer(staging_buffer, None);
        vulkan.device().free_memory(staging_memory, None);
    }
    // Shadow destroyed objects
    #[expect(unused)]
    let staging_buffer = ();
    #[expect(unused)]
    let staging_memory = ();

    let view = unsafe {
        let create_info = vk::ImageViewCreateInfo::default()
            .format(vk::Format::R16G16B16A16_SFLOAT)
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .layer_count(1)
                    .level_count(1),
            );
        vulkan
            .device()
            .create_image_view(&create_info, None)
            .unwrap()
    };

    let hdr_image = HdrImage {
        image,
        memory,
        view,
        extent: EXTENT,
    };

    let result = unsafe { hdr_scanner.scan(hdr_image).unwrap() };
    assert_eq!(f16::from_f32(result), MAXIMUM_VALUE);

    unsafe { hdr_image.destroy(&vulkan) };
}
