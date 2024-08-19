use ash::vk::{
    self, AccessFlags2, DependencyInfo, DeviceMemory, Extent2D, ExternalMemoryHandleTypeFlags,
    ExternalMemoryImageCreateInfo, Format, Image, ImageAspectFlags, ImageCreateInfo, ImageLayout,
    ImageMemoryBarrier2, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
    ImageViewCreateInfo, ImageViewType, ImportMemoryWin32HandleInfoKHR, MemoryAllocateInfo,
    MemoryDedicatedAllocateInfo, MemoryPropertyFlags, PipelineStageFlags2, SampleCountFlags,
    SharingMode, QUEUE_FAMILY_IGNORED,
};
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::{record_submit_command_buffer, VulkanInstance};
use windows_capture_provider::Capture;

use super::ActiveCapture;

impl ActiveCapture {
    /// Creates a vulkan image from the capture by importing the shared dx11 texture handle.
    pub fn image_from_capture(
        vk: &VulkanInstance,
        capture: &Capture,
    ) -> Result<(Image, DeviceMemory, ImageView), Error> {
        let _span = info_span!("image_from_capture").entered();

        // Create image with external memory flag.
        let image = unsafe {
            let image_extent = Extent2D {
                width: capture.display.size[0],
                height: capture.display.size[1],
            };

            let mut external_memory_image = ExternalMemoryImageCreateInfo::default()
                .handle_types(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

            let image_create_info = ImageCreateInfo {
                image_type: ImageType::TYPE_2D,
                format: Format::R16G16B16A16_SFLOAT,
                extent: image_extent.into(),
                mip_levels: 1,
                array_layers: 1,
                samples: SampleCountFlags::TYPE_1,
                tiling: ImageTiling::OPTIMAL,
                usage: ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED,
                sharing_mode: SharingMode::EXCLUSIVE,
                initial_layout: ImageLayout::UNDEFINED,
                ..Default::default()
            }
            .push_next(&mut external_memory_image);

            vk.device
                .create_image(&image_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating image"))?
        };

        // Create and import memory.
        let memory = unsafe {
            let memory_requirement = vk.device.get_image_memory_requirements(image);

            let memory_index = vk
                .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
                .ok_or(Error::NoSuitableMemoryType)?;

            let mut dedicated_allocation = MemoryDedicatedAllocateInfo::default().image(image);
            let mut import_info = ImportMemoryWin32HandleInfoKHR::default()
                .handle_type(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
                .handle(capture.handle.0 as isize);

            let allocate_info = MemoryAllocateInfo::default()
                .allocation_size(memory_requirement.size)
                .memory_type_index(memory_index)
                .push_next(&mut import_info)
                .push_next(&mut dedicated_allocation);

            let device_memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

            vk.device
                .bind_image_memory(image, device_memory, 0)
                .unwrap();

            device_memory
        };

        // transition image layout
        vk.record_submit_command_buffer(
            vk.command_buffer,
            vk.fence,
            &[],
            &[],
            |device, command_buffer| {
                let memory_barriers = [ImageMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::NONE,
                    dst_access_mask: AccessFlags2::NONE,
                    old_layout: ImageLayout::UNDEFINED,
                    new_layout: ImageLayout::GENERAL,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    image,
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
                Ok(())
            },
        )?;

        // Create the image view
        let image_view = unsafe {
            let image_view_create_info = ImageViewCreateInfo {
                image,
                view_type: ImageViewType::TYPE_2D,
                format: Format::R16G16B16A16_SFLOAT,
                subresource_range: ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };

            vk.device
                .create_image_view(&image_view_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating image view"))?
        };

        Ok((image, memory, image_view))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] record_submit_command_buffer::Error),
}
