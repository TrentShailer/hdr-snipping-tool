use ash::vk::{
    self, AccessFlags2, BufferImageCopy2, BufferUsageFlags, CopyImageToBufferInfo2, DependencyInfo,
    Extent2D, ImageAspectFlags, ImageLayout, ImageMemoryBarrier2, ImageSubresourceLayers,
    ImageSubresourceRange, MemoryMapFlags, MemoryPropertyFlags, Offset3D, PipelineStageFlags2,
    QUEUE_FAMILY_IGNORED, WHOLE_SIZE,
};

use thiserror::Error;
use vulkan_instance::{CommandBufferUsage, VulkanInstance};

use super::TonemapOutput;

impl TonemapOutput {
    /// Copies the contents of the image to a box.
    pub fn copy_to_box(&self, vk: &VulkanInstance) -> Result<Box<[u8]>, Error> {
        let data_length = self.size[0] as u64 * self.size[1] as u64 * 4;
        let (staging_buffer, staging_buffer_memory) = vk
            .create_bound_buffer(
                data_length,
                BufferUsageFlags::TRANSFER_DST,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )
            .map_err(|e| Error::Vulkan(e, "creating staging buffer"))?;

        vk.record_submit_command_buffer(
            CommandBufferUsage::Setup,
            &[],
            &[],
            |device, command_buffer| {
                let memory_barriers = [ImageMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::TRANSFER,
                    dst_access_mask: AccessFlags2::MEMORY_READ,
                    old_layout: ImageLayout::GENERAL,
                    new_layout: ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    image: self.image,
                    subresource_range: ImageSubresourceRange {
                        aspect_mask: ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                }];

                let dependency_info =
                    DependencyInfo::default().image_memory_barriers(&memory_barriers);

                unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

                let extent = Extent2D {
                    width: self.size[0],
                    height: self.size[1],
                };
                let image_subresource = ImageSubresourceLayers {
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                };
                let copy_regions = BufferImageCopy2 {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource,
                    image_offset: Offset3D::default(),
                    image_extent: extent.into(),
                    ..Default::default()
                };

                let regions = &[copy_regions];

                let image_copy_info = CopyImageToBufferInfo2::default()
                    .src_image(self.image)
                    .src_image_layout(ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .dst_buffer(staging_buffer)
                    .regions(regions);

                unsafe { device.cmd_copy_image_to_buffer2(command_buffer, &image_copy_info) };
                Ok(())
            },
        )?;

        unsafe {
            vk.device.wait_for_fences(
                &[*vk.fences.get(&CommandBufferUsage::Setup).unwrap()],
                true,
                u64::MAX,
            )
        }
        .map_err(|e| Error::Vulkan(e, "waiting for fence"))?;

        // Read staging buffer
        let data_box = unsafe {
            let memory_ptr = vk
                .device
                .map_memory(
                    staging_buffer_memory,
                    0,
                    WHOLE_SIZE,
                    MemoryMapFlags::empty(),
                )
                .map_err(|e| Error::Vulkan(e, "mapping staging buffer memory"))?;

            // Readback memory
            let data: &[u8] = std::slice::from_raw_parts(memory_ptr.cast(), data_length as usize);

            let data_box = Box::from(data);
            vk.device.unmap_memory(staging_buffer_memory);

            data_box
        };

        unsafe {
            vk.device.destroy_buffer(staging_buffer, None);
            vk.device.free_memory(staging_buffer_memory, None);
        }

        Ok(data_box)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordCommandBuffer(#[from] vulkan_instance::record_submit_command_buffer::Error),
}
