use ash::{
    khr::external_memory_win32,
    vk::{
        self, DeviceMemory, ExportMemoryAllocateInfo, Extent2D, ExternalMemoryHandleTypeFlags,
        ExternalMemoryImageCreateInfo, Format, Image, ImageCreateInfo, ImageLayout, ImageTiling,
        ImageType, ImageUsageFlags, MemoryAllocateInfo, MemoryGetWin32HandleInfoKHR,
        MemoryPropertyFlags, SampleCountFlags, SharingMode,
    },
};
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::VulkanInstance;

use super::ActiveCapture;

impl ActiveCapture {
    pub fn create_capture_image(
        vk: &VulkanInstance,
        image_size: [u32; 2],
    ) -> Result<(Image, DeviceMemory, ash::vk::HANDLE), Error> {
        let _span = info_span!("image_from_capture").entered();

        let image = unsafe {
            let image_extent = Extent2D {
                width: image_size[0],
                height: image_size[1],
            };

            let mut external_memory_image = ExternalMemoryImageCreateInfo::default()
                .handle_types(ExternalMemoryHandleTypeFlags::D3D11_TEXTURE);

            let image_create_info = ImageCreateInfo {
                image_type: ImageType::TYPE_2D,
                format: Format::R16G16B16A16_SFLOAT,
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
            }
            .push_next(&mut external_memory_image);

            vk.device
                .create_image(&image_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating image"))?
        };

        let image_memory = unsafe {
            let memory_requirement = vk.device.get_image_memory_requirements(image);

            let memory_index = vk
                .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
                .ok_or(Error::NoSuitableMemoryType)?;

            let mut export_allocate_info = ExportMemoryAllocateInfo::default()
                .handle_types(ExternalMemoryHandleTypeFlags::D3D11_TEXTURE);

            let allocate_info = MemoryAllocateInfo::default()
                .allocation_size(memory_requirement.size)
                .memory_type_index(memory_index)
                .push_next(&mut export_allocate_info);

            let device_memory = vk
                .device
                .allocate_memory(&allocate_info, None)
                .map_err(|e| Error::Vulkan(e, "allocating image memory"))?;

            device_memory
        };

        let handle = unsafe {
            let external_mem_device = external_memory_win32::Device::new(&vk.instance, &vk.device);
            let memory_handle_create_info = MemoryGetWin32HandleInfoKHR {
                memory: image_memory,
                handle_type: ExternalMemoryHandleTypeFlags::D3D11_TEXTURE,
                ..Default::default()
            };
            let handle = external_mem_device
                .get_memory_win32_handle(&memory_handle_create_info)
                .map_err(|e| Error::Vulkan(e, "getting win32 handle"))?;

            handle
        };

        Ok((image, image_memory, handle))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,
}
