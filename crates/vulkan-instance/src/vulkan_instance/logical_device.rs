use ash::{
    vk::{
        self, PhysicalDevice, PhysicalDevice16BitStorageFeatures,
        PhysicalDeviceDynamicRenderingFeatures, PhysicalDeviceFeatures2,
        PhysicalDeviceShaderFloat16Int8Features, PhysicalDeviceShaderSubgroupExtendedTypesFeatures,
        PhysicalDeviceSynchronization2Features, Queue,
    },
    Device, Instance,
};

use crate::vulkan_instance::physical_device;

use super::Error;

pub fn get_logical_device(
    instance: &Instance,
    physical_device: PhysicalDevice,
    queue_family_index: u32,
) -> Result<(Device, Queue), Error> {
    let queue_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&[1.0]);

    let device_extension_names_raw: Box<[*const i8]> = physical_device::REQUIRED_EXTENSIONS
        .into_iter()
        .map(|extension| extension.as_ptr())
        .collect();

    // Required features
    let mut shader_float16 =
        PhysicalDeviceShaderFloat16Int8Features::default().shader_float16(true);

    let mut storage_16_bit_access = PhysicalDevice16BitStorageFeatures::default()
        .storage_buffer16_bit_access(true)
        .uniform_and_storage_buffer16_bit_access(true);

    let mut subgroup_extended_types = PhysicalDeviceShaderSubgroupExtendedTypesFeatures::default()
        .shader_subgroup_extended_types(true);

    let mut dynamic_rendering =
        PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);

    let mut synchronization2 =
        PhysicalDeviceSynchronization2Features::default().synchronization2(true);

    let mut device_features = PhysicalDeviceFeatures2::default()
        .push_next(&mut shader_float16)
        .push_next(&mut storage_16_bit_access)
        .push_next(&mut subgroup_extended_types)
        .push_next(&mut dynamic_rendering)
        .push_next(&mut synchronization2);

    // end required features

    let device_create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_info))
        .enabled_extension_names(&device_extension_names_raw)
        .push_next(&mut device_features);

    let device = unsafe {
        instance
            .create_device(physical_device, &device_create_info, None)
            .map_err(|e| Error::Vulkan(e, "creating logical device"))?
    };

    let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

    Ok((device, queue))
}
