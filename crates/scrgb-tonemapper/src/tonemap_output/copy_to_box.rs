use std::u64;

use ash::vk::{
    self, AccessFlags2, BufferCreateInfo, BufferImageCopy2, BufferUsageFlags,
    CopyImageToBufferInfo2, DependencyFlags, DependencyInfo, Extent2D, ImageAspectFlags,
    ImageLayout, ImageMemoryBarrier2, ImageSubresourceLayers, ImageSubresourceRange,
    MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, Offset3D, PipelineStageFlags2,
    SharingMode, QUEUE_FAMILY_IGNORED, WHOLE_SIZE,
};
use smallvec::{smallvec, SmallVec};
use thiserror::Error;
use vulkan_instance::{CommandBufferUsage, VulkanInstance};

use super::TonemapOutput;

impl TonemapOutput {
    /// Copies the contents of the image to a box.
    pub fn copy_to_box(&self, vk: &VulkanInstance) -> Result<Box<[u8]>, Error> {
        let data_length = self.size[0] as u64 * self.size[1] as u64 * 4;
        let (staging_buffer, staging_buffer_memory) = unsafe {
            let buffer_create_info = BufferCreateInfo {
                size: data_length,
                usage: BufferUsageFlags::TRANSFER_DST,
                sharing_mode: SharingMode::EXCLUSIVE,
                ..Default::default()
            };

            let buffer = vk
                .device
                .create_buffer(&buffer_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating staging buffer"))?;

            let memory_requirements = vk.device.get_buffer_memory_requirements(buffer);

            let memory_index = vk
                .find_memorytype_index(
                    &memory_requirements,
                    MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
                )
                .ok_or(Error::NoSuitableMemoryType)?;

            let allocate_info = MemoryAllocateInfo {
                allocation_size: memory_requirements.size,
                memory_type_index: memory_index,
                ..Default::default()
            };

            let memory = vk
                .device
                .allocate_memory(&allocate_info, None)
                .map_err(|e| Error::Vulkan(e, "allocating staging buffer"))?;

            vk.device
                .bind_buffer_memory(buffer, memory, 0)
                .map_err(|e| Error::Vulkan(e, "binding staging memory"))?;

            (buffer, memory)
        };

        vk.record_submit_command_buffer(
            CommandBufferUsage::Setup,
            &[],
            &[],
            |device, command_buffer| {
                let subresource_range = ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 1,
                    level_count: 1,
                    base_array_layer: 1,
                    layer_count: 1,
                };

                let memory_barrier = ImageMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::TRANSFER,
                    dst_access_mask: AccessFlags2::MEMORY_READ,
                    old_layout: ImageLayout::GENERAL,
                    new_layout: ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    image: self.image,
                    subresource_range,
                    ..Default::default()
                };
                let image_barriers: SmallVec<[_; 1]> = smallvec![memory_barrier];

                let dependency_info = DependencyInfo {
                    dependency_flags: DependencyFlags::BY_REGION,
                    memory_barrier_count: 0,
                    p_memory_barriers: std::ptr::null(),
                    buffer_memory_barrier_count: 0,
                    p_buffer_memory_barriers: std::ptr::null(),
                    image_memory_barrier_count: 1,
                    p_image_memory_barriers: image_barriers.as_ptr(),
                    ..Default::default()
                };

                unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

                let extent = Extent2D {
                    width: self.size[0],
                    height: self.size[1],
                };
                let image_subresource = ImageSubresourceLayers {
                    aspect_mask: ImageAspectFlags::COLOR,
                    mip_level: 1,
                    base_array_layer: 1,
                    layer_count: 1,
                };
                let copy_regions = BufferImageCopy2 {
                    buffer_offset: 0,
                    buffer_row_length: self.size[0],
                    buffer_image_height: self.size[1],
                    image_subresource,
                    image_offset: Offset3D::default(),
                    image_extent: extent.into(),
                    ..Default::default()
                };
                let regions: SmallVec<[_; 1]> = smallvec![copy_regions];
                let image_copy_info = CopyImageToBufferInfo2 {
                    src_image: self.image,
                    src_image_layout: ImageLayout::TRANSFER_SRC_OPTIMAL,
                    dst_buffer: staging_buffer,
                    region_count: 1,
                    p_regions: regions.as_ptr(),
                    ..Default::default()
                };
                unsafe { device.cmd_copy_image_to_buffer2(command_buffer, &image_copy_info) };
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

        // Map memory
        let memory_ptr = unsafe {
            vk.device.map_memory(
                staging_buffer_memory,
                0,
                WHOLE_SIZE,
                MemoryMapFlags::empty(),
            )
        }
        .map_err(|e| Error::Vulkan(e, "mapping staging buffer memory"))?;

        // Readback memory
        let data: &[u8] =
            unsafe { std::slice::from_raw_parts(memory_ptr.cast(), data_length as usize) };
        let data_box = Box::from(data);

        // unmap memory and cleanup
        unsafe {
            vk.device.unmap_memory(staging_buffer_memory);
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
