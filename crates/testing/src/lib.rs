use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use core::slice;
use std::{fs::File, io::Read};

use ash::{util::Align, vk};
use ash_helper::{
    allocate_buffer, allocate_image, cmd_transition_image, onetime_command, VulkanContext,
};
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};
use tracing::{
    info, info_span,
    subscriber::{set_global_default, SetGlobalDefaultError},
    Level,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt};
use vulkan::{HdrImage, QueuePurpose, Vulkan};

#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub width: u32,
    pub height: u32,
    pub sdr_white: f32,
}

pub fn setup_logger() -> Result<WorkerGuard, SetGlobalDefaultError> {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(Level::TRACE)
        .with_target("winit", Level::WARN);

    // stdout logger
    let (std_writer, std_guard) = tracing_appender::non_blocking(std::io::stdout());
    let std_logger = tracing_subscriber::fmt::layer()
        .with_writer(std_writer)
        .with_ansi(false)
        .with_target(false)
        .with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER);

    // Register loggers
    let collector = tracing_subscriber::registry().with(std_logger).with(filter);

    set_global_default(collector)?;

    info!("Application Start");
    Ok(std_guard)
}

pub fn load_hdr_capture(vulkan: &Vulkan) -> (Metadata, HdrImage) {
    let _span = info_span!("LoadHdrCapture").entered();
    let (data, metadata) = {
        let mut metadata_file = File::open("crates/testing/src/assets/metadata.toml").unwrap();
        let mut metadata_str = String::new();
        metadata_file.read_to_string(&mut metadata_str).unwrap();
        let metadata: Metadata = toml::de::from_str(&metadata_str).unwrap();

        let data_file = File::open("crates/testing/src/assets/hdr-capture.raw").unwrap();
        let mut data_file = ZlibDecoder::new(data_file);
        let mut data = vec![];
        data_file.read_to_end(&mut data).unwrap();

        (data, metadata)
    };

    let extent = vk::Extent2D::default()
        .width(metadata.width)
        .height(metadata.height);

    let (image, image_memory) = {
        let queue_family = vulkan.queue_family_index();
        let image_create_info = vk::ImageCreateInfo::default()
            .array_layers(1)
            .extent(extent.into())
            .format(vk::Format::R16G16B16A16_SFLOAT)
            .image_type(vk::ImageType::TYPE_2D)
            .flags(vk::ImageCreateFlags::empty())
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .mip_levels(1)
            .queue_family_indices(slice::from_ref(&queue_family))
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(
                vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::SAMPLED,
            );

        let (image, memory, _) = unsafe {
            allocate_image(
                vulkan,
                &image_create_info,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                "HDR",
            )
            .unwrap()
        };

        (image, memory)
    };

    let (staging_buffer, staging_memory) = {
        let queue_family = vulkan.queue_family_index();

        let buffer_info = vk::BufferCreateInfo::default()
            .queue_family_indices(slice::from_ref(&queue_family))
            .usage(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST)
            .size(data.len() as u64);

        let (buffer, memory, _) = unsafe {
            allocate_buffer(
                vulkan,
                &buffer_info,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                "Staging",
            )
            .unwrap()
        };

        (buffer, memory)
    };

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
                        data.len() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap()
            };

            let mut align: Align<u8> =
                unsafe { Align::new(pointer, align_of::<u8>() as u64, data.len() as u64) };
            align.copy_from_slice(&data);

            unsafe { vulkan.device().unmap_memory(staging_memory) };
        }

        unsafe {
            onetime_command(
                vulkan,
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
                            .buffer_image_height(extent.height)
                            .buffer_row_length(extent.width)
                            .buffer_offset(0)
                            .image_extent(extent.into())
                            .image_subresource(subresource);

                        vk.device().cmd_copy_buffer_to_image(
                            command_buffer,
                            staging_buffer,
                            image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            slice::from_ref(&image_copy),
                        );
                    }

                    // Transition to Shader Read Optimal
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

    unsafe {
        vulkan.device().destroy_buffer(staging_buffer, None);
        vulkan.device().free_memory(staging_memory, None);
    }

    (
        metadata,
        HdrImage {
            image,
            memory: image_memory,
            view: image_view,
            extent,
        },
    )
}
