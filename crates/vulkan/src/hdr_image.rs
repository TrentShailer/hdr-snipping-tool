use std::time::Instant;

use ash::vk;
use ash_helper::{
    AllocationError, VkError, VulkanContext, cmd_transition_image, find_memorytype_index,
    onetime_command,
};
use tracing::debug;

use crate::{QueuePurpose, Vulkan};

#[derive(Copy, Clone)]
/// An HDR Image in `R16G16B16A16_SFLOAT` format.
pub struct HdrImage {
    /// The Vulkan image.
    pub image: vk::Image,

    /// The image memory.
    pub memory: vk::DeviceMemory,

    /// The image view.
    pub view: vk::ImageView,

    /// The image extent.
    pub extent: vk::Extent2D,
}

impl HdrImage {
    /// Imports a capture taken using the Windows Graphics Capture API using a shared handle to the
    /// capture.
    pub unsafe fn import_windows_capture(
        vulkan: &Vulkan,
        size: [u32; 2],
        handle: isize,
    ) -> Result<Self, AllocationError> {
        let start = Instant::now();

        let extent = vk::Extent2D::default().width(size[0]).height(size[1]);

        // Create Image
        let image = unsafe {
            let mut external_memory_image = vk::ExternalMemoryImageCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

            let image_create_info = vk::ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .format(vk::Format::R16G16B16A16_SFLOAT)
                .extent(extent.into())
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .push_next(&mut external_memory_image);

            vulkan
                .device()
                .create_image(&image_create_info, None)
                .map_err(|e| VkError::new(e, "vkCreateImage"))?
        };

        // Create and import memory.
        let (memory, _) = unsafe {
            let memory_requirements = vulkan.device().get_image_memory_requirements(image);

            let memory_index = find_memorytype_index(
                vulkan,
                memory_requirements,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .ok_or(AllocationError::NoSuitableMemoryType)?;

            let mut dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let mut import_info = vk::ImportMemoryWin32HandleInfoKHR::default()
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
                .handle(handle);

            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(memory_requirements.size)
                .memory_type_index(memory_index)
                .push_next(&mut import_info)
                .push_next(&mut dedicated_allocation);

            let device_memory = vulkan
                .device()
                .allocate_memory(&allocate_info, None)
                .map_err(|e| VkError::new(e, "vkAllocateMemory"))?;

            (device_memory, memory_requirements)
        };

        // Bind image to memory
        unsafe {
            vulkan
                .device()
                .bind_image_memory(image, memory, 0)
                .map_err(|e| VkError::new(e, "vkBindImageMemory"))?;
        }

        // Create image view
        let view = unsafe {
            let create_info = vk::ImageViewCreateInfo::default()
                .format(vk::Format::R16G16B16A16_SFLOAT)
                .view_type(vk::ImageViewType::TYPE_2D)
                .image(image)
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
                .map_err(|e| VkError::new(e, "vkCreateImageView"))?
        };

        // transition image layout
        unsafe {
            onetime_command(
                vulkan,
                vulkan.transient_pool(),
                vulkan.queue(QueuePurpose::Compute),
                |vulkan, command_buffer| {
                    cmd_transition_image(
                        vulkan,
                        command_buffer,
                        image,
                        vk::ImageLayout::UNDEFINED,
                        vk::ImageLayout::GENERAL,
                    )
                    .unwrap();
                },
                "Transition Capture",
            )?;
        }

        debug!(
            "Importing Windows capture took {}ms",
            start.elapsed().as_millis()
        );

        Ok(Self {
            image,
            memory,
            view,
            extent,
        })
    }

    /// Destroy the image.
    pub unsafe fn destroy(self, vulkan: &Vulkan) {
        unsafe {
            vulkan.device().destroy_image_view(self.view, None);
            vulkan.device().destroy_image(self.image, None);
            vulkan.device().free_memory(self.memory, None);
        }
    }
}
