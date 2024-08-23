use ash::vk::{
    DependencyInfo, Extent2D, ExternalMemoryHandleTypeFlags, ExternalMemoryImageCreateInfo, Format,
    ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageSubresourceRange, ImageTiling, ImageType,
    ImageUsageFlags, ImageViewCreateInfo, ImageViewType, ImportMemoryWin32HandleInfoKHR,
    MemoryAllocateInfo, MemoryDedicatedAllocateInfo, MemoryPropertyFlags, SampleCountFlags,
    SharingMode,
};
use half::f16;
use tracing::{info, instrument};
use vulkan_instance::{VulkanError, VulkanInstance};
use windows_capture_provider::WindowsCapture;

use crate::Maximum;

use super::{Error, HdrCapture};

impl<'d> HdrCapture<'d> {
    /// Create an Hdr Capture by importing a windows capture
    #[instrument("HdrCapture::import_windows_capture", skip_all, err)]
    pub fn import_windows_capture(
        vk: &'d VulkanInstance,
        maximum: &Maximum,
        capture: &WindowsCapture,
        hdr_whitepoint: f32,
    ) -> Result<Self, Error> {
        // Create image with external memory flag.
        let image = unsafe {
            let image_extent = Extent2D {
                width: capture.size[0],
                height: capture.size[1],
            };

            let mut external_memory_image = ExternalMemoryImageCreateInfo::default()
                .handle_types(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

            let image_create_info = ImageCreateInfo::default()
                .image_type(ImageType::TYPE_2D)
                .format(Format::R16G16B16A16_SFLOAT)
                .extent(image_extent.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(SampleCountFlags::TYPE_1)
                .tiling(ImageTiling::OPTIMAL)
                .usage(ImageUsageFlags::SAMPLED | ImageUsageFlags::STORAGE)
                .sharing_mode(SharingMode::EXCLUSIVE)
                .initial_layout(ImageLayout::UNDEFINED)
                .push_next(&mut external_memory_image);

            vk.device
                .create_image(&image_create_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating image"))?
        };

        // Create and import memory.
        let memory = unsafe {
            let memory_requirement = vk.device.get_image_memory_requirements(image);

            let memory_index = vk
                .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
                .ok_or(VulkanError::NoSuitableMemoryType)?;

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
            &[],
            &[],
            |device, command_buffer| unsafe {
                let memory_barriers = [VulkanInstance::image_memory_barrier()
                    .old_layout(ImageLayout::UNDEFINED)
                    .new_layout(ImageLayout::GENERAL)
                    .image(image)];

                let dependency_info =
                    DependencyInfo::default().image_memory_barriers(&memory_barriers);

                device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
                Ok(())
            },
        )?;

        // Create the image view
        let image_view = unsafe {
            let image_view_create_info = ImageViewCreateInfo::default()
                .image(image)
                .view_type(ImageViewType::TYPE_2D)
                .format(Format::R16G16B16A16_SFLOAT)
                .subresource_range(ImageSubresourceRange {
                    aspect_mask: ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            vk.device
                .create_image_view(&image_view_create_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating image view"))?
        };

        let mut hdr_capture = Self {
            device: &vk.device,
            image,
            memory,
            image_view,
            size: capture.size,
            whitepoint: 0.0,
        };

        let maximum = maximum.find_maximum(vk, &hdr_capture)?;
        let maximum = if f16::from_bits(maximum.to_bits() - 1).to_f32()
            == capture.display.sdr_referece_white
        {
            capture.display.sdr_referece_white
        } else {
            maximum.to_f32()
        };

        info!("Maximum: {:.2}", maximum);

        let whitepoint = if maximum > capture.display.sdr_referece_white {
            hdr_whitepoint
        } else {
            capture.display.sdr_referece_white
        };

        info!("Whitepoint: {:.2}", whitepoint);

        hdr_capture.whitepoint = whitepoint;

        Ok(hdr_capture)
    }
}
