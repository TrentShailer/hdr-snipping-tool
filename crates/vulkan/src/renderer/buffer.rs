use core::slice;

use ash::{util::Align, vk};
use ash_helper::{
    BufferAlignment, BufferUsageFlags, VkError, VulkanContext, allocate_buffer, onetime_command,
};

use crate::{QueuePurpose, Vulkan};

use super::{
    CreationError,
    pipelines::{
        CapturePipeline, capture_pipeline,
        line_pipeline::{self, LinePipeline},
        selection_pipeline::{self, SelectionPipeline},
    },
};

/// A Wrapper around the buffer containing all fo the verticies, indicies, and instance data.
#[derive(Clone, Copy)]
pub struct RenderBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,

    /// The offset in the buffer for the pipelines' verticies/indicies/instance data.
    pub line_offset: u64,
    pub selection_offset: u64,
    pub capture_offset: u64,
}

impl RenderBuffer {
    /// Create and initialise the data in the RenderBuffer.
    pub fn new(vulkan: &Vulkan) -> Result<Self, CreationError> {
        let alignment = BufferAlignment::new(vulkan);
        let usage = BufferUsageFlags::empty();
        // Plan buffer slices
        let (
            buffer_size,
            (line_offset, line_end),
            (selection_offset, selection_end),
            (capture_offset, capture_end),
        ) = {
            let (line_offset, line_end) = alignment.calc_slice(
                0,
                align_of::<line_pipeline::Vertex>() as u64,
                size_of::<line_pipeline::Vertex>() as u64,
                LinePipeline::VERTICIES.len() as u64,
                usage,
            );

            let (selection_offset, selection_end) = alignment.calc_slice(
                line_end,
                align_of::<selection_pipeline::Vertex>() as u64,
                size_of::<selection_pipeline::Vertex>() as u64,
                SelectionPipeline::VERTICIES.len() as u64,
                usage,
            );

            let (capture_offset, capture_end) = alignment.calc_slice(
                selection_end,
                align_of::<capture_pipeline::Vertex>() as u64,
                size_of::<capture_pipeline::Vertex>() as u64,
                CapturePipeline::VERTICIES.len() as u64,
                usage,
            );

            let buffer_size = capture_end;

            (
                buffer_size,
                (line_offset, line_end),
                (selection_offset, selection_end),
                (capture_offset, capture_end),
            )
        };

        // Allocate the buffer
        let (buffer, memory, _) = {
            let queue_family = vulkan.queue_family_index();
            let buffer_create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(&queue_family))
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(buffer_size)
                .usage(
                    vk::BufferUsageFlags::VERTEX_BUFFER
                        | vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_DST,
                );

            unsafe {
                allocate_buffer(
                    vulkan,
                    &buffer_create_info,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    "Render",
                )?
            }
        };

        // Create staging buffer
        let (staging_buffer, staging_memory, _) = {
            let queue_family = vulkan.queue_family_index();
            let buffer_create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(&queue_family))
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .size(buffer_size)
                .usage(vk::BufferUsageFlags::TRANSFER_SRC);

            unsafe {
                allocate_buffer(
                    vulkan,
                    &buffer_create_info,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    "Render Staging",
                )?
            }
        };

        // Write data to staging
        {
            // Write line verticies
            {
                let pointer = unsafe {
                    vulkan
                        .device()
                        .map_memory(
                            staging_memory,
                            line_offset,
                            line_end - line_offset,
                            vk::MemoryMapFlags::empty(),
                        )
                        .map_err(|e| VkError::new(e, "vkMapMemory"))?
                };

                let mut align = unsafe {
                    Align::new(
                        pointer,
                        align_of::<line_pipeline::Vertex>() as u64,
                        line_end - line_offset,
                    )
                };

                align.copy_from_slice(&LinePipeline::VERTICIES);

                unsafe { vulkan.device().unmap_memory(staging_memory) };
            }

            // Write selection verticies
            {
                let pointer = unsafe {
                    vulkan
                        .device()
                        .map_memory(
                            staging_memory,
                            selection_offset,
                            selection_end - selection_offset,
                            vk::MemoryMapFlags::empty(),
                        )
                        .map_err(|e| VkError::new(e, "vkMapMemory"))?
                };

                let mut align = unsafe {
                    Align::new(
                        pointer,
                        align_of::<selection_pipeline::Vertex>() as u64,
                        selection_end - selection_offset,
                    )
                };

                align.copy_from_slice(&SelectionPipeline::VERTICIES);

                unsafe { vulkan.device().unmap_memory(staging_memory) };
            }

            // Write capture verticies
            {
                let pointer = unsafe {
                    vulkan
                        .device()
                        .map_memory(
                            staging_memory,
                            capture_offset,
                            capture_end - capture_offset,
                            vk::MemoryMapFlags::empty(),
                        )
                        .map_err(|e| VkError::new(e, "vkMapMemory"))?
                };

                let mut align = unsafe {
                    Align::new(
                        pointer,
                        align_of::<capture_pipeline::Vertex>() as u64,
                        capture_end - capture_offset,
                    )
                };

                align.copy_from_slice(&CapturePipeline::VERTICIES);

                unsafe { vulkan.device().unmap_memory(staging_memory) };
            }
        }

        // Copy data to GPU
        unsafe {
            onetime_command(
                vulkan,
                vulkan.transient_pool(),
                vulkan.queue(QueuePurpose::Graphics),
                |vk, command_buffer| {
                    let region = vk::BufferCopy::default()
                        .src_offset(0)
                        .dst_offset(0)
                        .size(buffer_size);

                    vk.device().cmd_copy_buffer(
                        command_buffer,
                        staging_buffer,
                        buffer,
                        slice::from_ref(&region),
                    );
                },
                "Upload Render Data",
            )?;
        }

        // Cleanup
        unsafe {
            vulkan.device().destroy_buffer(staging_buffer, None);
            vulkan.device().free_memory(staging_memory, None);
        }

        Ok(Self {
            buffer,
            memory,

            line_offset,
            selection_offset,
            capture_offset,
        })
    }
}
