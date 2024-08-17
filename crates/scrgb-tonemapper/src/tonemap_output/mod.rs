pub mod copy_to_box;

use std::sync::Arc;

use ash::{
    vk::{
        self, AccessFlags2, DependencyInfo, DeviceMemory, Extent2D, Format, Image,
        ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier2, ImageSubresourceRange,
        ImageTiling, ImageType, ImageUsageFlags, ImageView, ImageViewCreateInfo, ImageViewType,
        MemoryAllocateInfo, MemoryPropertyFlags, PipelineStageFlags2, SampleCountFlags,
        SharingMode, QUEUE_FAMILY_IGNORED,
    },
    Device,
};
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::{record_submit_command_buffer, CommandBufferUsage};

use crate::VulkanInstance;

/// Output from the tonemapper.\
/// Vulkan image and associated values.
pub struct TonemapOutput {
    pub image: Image,
    pub memory: DeviceMemory,
    pub image_view: ImageView,
    pub size: [u32; 2],
    pub device: Arc<Device>,
}

impl TonemapOutput {
    /// Create an empty tonemap output.
    pub fn new(vk: &VulkanInstance, size: [u32; 2]) -> Result<Self, Error> {
        let _span = info_span!("TonemapOutput::new").entered();

        let image = unsafe {
            let image_extent = Extent2D {
                width: size[0],
                height: size[1],
            };

            let image_create_info = ImageCreateInfo {
                image_type: ImageType::TYPE_2D,
                format: Format::R8G8B8A8_UNORM,
                extent: image_extent.into(),
                mip_levels: 1,
                array_layers: 1,
                samples: SampleCountFlags::TYPE_1,
                tiling: ImageTiling::OPTIMAL,
                usage: ImageUsageFlags::TRANSFER_SRC
                    | ImageUsageFlags::TRANSFER_DST
                    | ImageUsageFlags::STORAGE,
                sharing_mode: SharingMode::EXCLUSIVE,
                initial_layout: ImageLayout::UNDEFINED,
                ..Default::default()
            };

            vk.device
                .create_image(&image_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating image"))?
        };

        let image_memory = unsafe {
            let memory_requirement = vk.device.get_image_memory_requirements(image);

            let memory_index = vk
                .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
                .ok_or(Error::NoSuitableMemoryType)?;

            let allocate_info = MemoryAllocateInfo {
                allocation_size: memory_requirement.size,
                memory_type_index: memory_index,
                ..Default::default()
            };

            let device_memory = vk
                .device
                .allocate_memory(&allocate_info, None)
                .map_err(|e| Error::Vulkan(e, "allocating memory"))?;

            device_memory
        };

        unsafe {
            vk.device
                .bind_image_memory(image, image_memory, 0)
                .map_err(|e| Error::Vulkan(e, "binding memory"))?
        };

        vk.record_submit_command_buffer(
            CommandBufferUsage::Tonemap,
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
            },
        )?;

        let image_view = unsafe {
            let image_view_create_info = ImageViewCreateInfo {
                image,
                view_type: ImageViewType::TYPE_2D,
                format: Format::R8G8B8A8_UNORM,
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

        Ok(Self {
            image,
            image_view,
            memory: image_memory,
            size,
            device: vk.device.clone(),
        })
    }
}

impl Drop for TonemapOutput {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.free_memory(self.memory, None);
            self.device.destroy_image(self.image, None);
        }
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
