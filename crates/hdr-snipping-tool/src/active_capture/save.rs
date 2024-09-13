use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufWriter},
};

use arboard::{Clipboard, ImageData};
use ash::vk::{
    AccessFlags2, BufferImageCopy2, BufferUsageFlags, CopyImageToBufferInfo2, DependencyInfo,
    Extent2D, ImageAspectFlags, ImageLayout, ImageSubresourceLayers, MemoryPropertyFlags,
    PipelineStageFlags2,
};
use chrono::Local;
use hdr_capture::tonemap;
use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, ImageError, Rgba};
use thiserror::Error;
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::{active_app::ActiveApp, project_directory};

use super::ActiveCapture;

impl ActiveCapture {
    #[instrument("ActiveCapture::Capture", skip_all, err)]
    pub fn save(&mut self, app: &ActiveApp) -> Result<(), Error> {
        let ActiveApp { vk, tonemap, .. } = app;

        let size = self.capture.size;
        let tonemapped_image = tonemap.tonemap(&self.capture)?;
        let raw_capture: Box<[u8]> = unsafe {
            let buffer_size = size[0] as u64 * size[1] as u64 * 4;
            let (staging_buffer, staging_memory) = vk.create_bound_buffer(
                buffer_size,
                BufferUsageFlags::TRANSFER_DST,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )?;

            vk.record_submit_command_buffer(
                vk.command_buffer,
                &[],
                &[],
                |device, command_buffer| {
                    let memory_barriers = [VulkanInstance::image_memory_barrier()
                        .dst_stage_mask(PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(AccessFlags2::MEMORY_READ)
                        .old_layout(ImageLayout::GENERAL)
                        .new_layout(ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .image(tonemapped_image.0)];

                    let dependency_info =
                        DependencyInfo::default().image_memory_barriers(&memory_barriers);

                    device.cmd_pipeline_barrier2(command_buffer, &dependency_info);

                    let extent = Extent2D {
                        width: size[0],
                        height: size[1],
                    };
                    let image_subresource = ImageSubresourceLayers::default()
                        .layer_count(1)
                        .aspect_mask(ImageAspectFlags::COLOR);

                    let regions = [BufferImageCopy2::default()
                        .image_subresource(image_subresource)
                        .image_extent(extent.into())];

                    let image_copy_info = CopyImageToBufferInfo2::default()
                        .src_image(tonemapped_image.0)
                        .src_image_layout(ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .dst_buffer(staging_buffer)
                        .regions(&regions);

                    device.cmd_copy_image_to_buffer2(command_buffer, &image_copy_info);

                    Ok(())
                },
            )?;

            vk.device
                .wait_for_fences(&[vk.command_buffer.1], true, u64::MAX)
                .map_err(|e| VulkanError::VkResult(e, "waiting for fence"))?;

            let raw_slice: &[u8] = vk.read_from_memory(staging_memory, 0, buffer_size)?;

            vk.device.destroy_buffer(staging_buffer, None);
            vk.device.free_memory(staging_memory, None);

            Box::from(raw_slice)
        };

        let raw_capture_len = raw_capture.len();

        let img: ImageBuffer<Rgba<u8>, Box<[u8]>> =
            match ImageBuffer::from_raw(size[0], size[1], raw_capture) {
                Some(img) => img,
                None => return Err(Error::ImageBuffer(size[0], size[1], raw_capture_len)),
            };

        // Get selection view
        let selection_pos = self.selection.rect.left_top();
        let selection_size = self.selection.rect.size();

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = img
            .view(
                selection_pos[0],
                selection_pos[1],
                selection_size[0],
                selection_size[1],
            )
            .to_image();

        // Write image to file
        let name = format!("screenshot {}.png", Local::now().format("%F %H-%M-%S"));
        let path = project_directory().join(name);
        let file = File::create(path).map_err(Error::CreateFile)?;
        let mut buffer = BufWriter::new(file);
        let encoder = PngEncoder::new(&mut buffer);
        img.write_with_encoder(encoder)?;

        // Set clipboard
        let mut clipboard = Clipboard::new().map_err(Error::ClipboardInstance)?;
        clipboard
            .set_image(ImageData {
                width: selection_size[0] as usize,
                height: selection_size[1] as usize,
                bytes: Cow::Borrowed(img.as_raw()),
            })
            .map_err(Error::ClipboardSave)?;

        unsafe {
            vk.device.destroy_image_view(tonemapped_image.2, None);
            vk.device.destroy_image(tonemapped_image.0, None);
            vk.device.free_memory(tonemapped_image.1, None);
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Vulkan(#[from] VulkanError),

    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] tonemap::Error),

    #[error("Failed to create image buffer:\nTexture Size: {0}, {1}\nCapture Data: {2}")]
    ImageBuffer(u32, u32, usize),

    #[error("Failed to create file for capture:\n{0}")]
    CreateFile(#[source] io::Error),

    #[error("Failed to write capture to file:\n{0}")]
    WriteFile(#[from] ImageError),

    #[error("Failed to get an clipboard instance:\n{0}")]
    ClipboardInstance(#[source] arboard::Error),

    #[error("Failed to save the capture in the clipboard:\n{0}")]
    ClipboardSave(#[source] arboard::Error),
}
