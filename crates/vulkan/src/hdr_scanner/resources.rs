use core::slice;

use ash::vk;
use ash_helper::{allocate_buffer, BufferAlignment, VulkanContext};
use half::f16;

use crate::{
    hdr_scanner::{BufferScanner, ImageScanner},
    Vulkan,
};

use super::Error;

pub struct Resources {
    pub extent: vk::Extent2D,

    pub subgroup_size: u32,

    pub memory: vk::DeviceMemory,
    pub buffer: vk::Buffer,

    pub read_size: u64,
    pub write_offset: u64,
    pub write_size: u64,

    pub result_in_read: bool,
}

impl Resources {
    pub unsafe fn new(vulkan: &Vulkan, extent: vk::Extent2D) -> Result<Self, Error> {
        let subgroup_size = {
            let mut subgroup_properties = vk::PhysicalDeviceSubgroupProperties::default();

            let mut physical_device_properties =
                vk::PhysicalDeviceProperties2::default().push_next(&mut subgroup_properties);

            vulkan.instance().get_physical_device_properties2(
                vulkan.physical_device(),
                &mut physical_device_properties,
            );

            subgroup_properties.subgroup_size
        };

        let (read_size, write_offset, write_end) = {
            let image_outputs = ImageScanner::output_count(extent, subgroup_size);
            let read_size = image_outputs as u64 * size_of::<f16>() as u64;

            let alignment = BufferAlignment::new(vulkan);
            let buffer_outputs = BufferScanner::output_count(image_outputs, subgroup_size);

            let (write_offset, write_end) = alignment.calc_slice(
                read_size,
                align_of::<f16>() as u64,
                size_of::<f16>() as u64,
                buffer_outputs as u64,
                ash_helper::BufferUsageFlags::STORAGE_BUFFER
                    | ash_helper::BufferUsageFlags::MEMORY_MAP,
            );

            (read_size, write_offset, write_end)
        };

        let (buffer, memory) = {
            let buffer_size = write_end;
            let queue_family = vulkan.queue_family_index();

            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(&queue_family))
                .size(buffer_size)
                .usage(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC);

            let (buffer, memory, _) = unsafe {
                allocate_buffer(
                    vulkan,
                    &create_info,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    "HDR Scanner Resources",
                )?
            };

            (buffer, memory)
        };

        Ok(Self {
            extent,
            subgroup_size,

            memory,
            buffer,

            read_size,
            write_offset,
            write_size: write_end - write_offset,

            result_in_read: false,
        })
    }

    pub unsafe fn destory(self, vulkan: &Vulkan) {
        vulkan.device().destroy_buffer(self.buffer, None);
        vulkan.device().free_memory(self.memory, None);
    }
}
