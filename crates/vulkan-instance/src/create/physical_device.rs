use std::ffi::{self, CStr};

use ash::{
    khr::{self, surface},
    vk::{
        self, PhysicalDevice, PhysicalDevice16BitStorageFeatures,
        PhysicalDeviceDynamicRenderingFeatures, PhysicalDeviceFeatures2,
        PhysicalDeviceShaderFloat16Int8Features, PhysicalDeviceShaderSubgroupExtendedTypesFeatures,
        PhysicalDeviceSynchronization2Features, SurfaceKHR,
    },
    Instance,
};
use tracing::{error, info, instrument};

use crate::VulkanError;

use super::Error;

// -----

pub const REQUIRED_EXTENSIONS: [&ffi::CStr; 3] = [
    khr::swapchain::NAME,
    khr::external_memory_win32::NAME,
    khr::swapchain_mutable_format::NAME,
];

// -----
#[instrument(skip_all, err)]
pub fn get_physical_device(
    instance: &Instance,
    surface: SurfaceKHR,
    surface_loader: &surface::Instance,
) -> Result<(PhysicalDevice, u32), Error> {
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .map_err(|e| VulkanError::VkResult(e, "enumerating physical devices"))?
    };

    let (physical_device, queue_family_index) = physical_devices
        .iter()
        .find_map(|physical_device| {
            if !supports_required_extensions(instance, *physical_device) {
                return None;
            }

            if !supports_required_features(instance, *physical_device) {
                return None;
            }

            let queue_family_index =
                match find_valid_queue(instance, *physical_device, surface, surface_loader) {
                    Some(queue_family_index) => queue_family_index,
                    None => return None,
                };

            Some((*physical_device, queue_family_index))
        })
        .ok_or(Error::NoSuitableDevices)?;

    let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let device_name = device_properties
        .device_name_as_c_str()
        .unwrap_or(unsafe { CStr::from_bytes_with_nul_unchecked(b"Invalid name\0") })
        .to_string_lossy();
    let device_type = device_properties.device_type;

    info!("Physical Device: {} ({:#?})", device_name, device_type);

    Ok((physical_device, queue_family_index))
}

fn supports_required_extensions(instance: &Instance, device: PhysicalDevice) -> bool {
    let extension_properties = unsafe {
        match instance.enumerate_device_extension_properties(device) {
            Ok(extension_properties) => extension_properties,
            Err(e) => {
                error!("Failed to enumerate device extension properties {e}");
                return false;
            }
        }
    };

    let extension_names: Box<[ffi::CString]> = extension_properties
        .into_iter()
        .filter_map(
            |extension_property| match extension_property.extension_name_as_c_str() {
                Ok(name) => Some(name.to_owned()),
                Err(_) => None,
            },
        )
        .collect();

    REQUIRED_EXTENSIONS
        .into_iter()
        .all(|required_extension_name| {
            extension_names
                .iter()
                .any(|extension_name| extension_name == &required_extension_name.into())
        })
}

fn supports_required_features(instance: &Instance, device: PhysicalDevice) -> bool {
    let mut synchronization2 = PhysicalDeviceSynchronization2Features::default();
    let mut shader_float16 = PhysicalDeviceShaderFloat16Int8Features::default();
    let mut storage_16_bit_access = PhysicalDevice16BitStorageFeatures::default();
    let mut subgroup_extended_types = PhysicalDeviceShaderSubgroupExtendedTypesFeatures::default();
    let mut dynamic_rendering = PhysicalDeviceDynamicRenderingFeatures::default();

    let mut device_features = PhysicalDeviceFeatures2::default()
        .push_next(&mut shader_float16)
        .push_next(&mut storage_16_bit_access)
        .push_next(&mut subgroup_extended_types)
        .push_next(&mut dynamic_rendering)
        .push_next(&mut synchronization2);

    unsafe { instance.get_physical_device_features2(device, &mut device_features) };

    shader_float16.shader_float16 == vk::TRUE
        && storage_16_bit_access.storage_buffer16_bit_access == vk::TRUE
        && storage_16_bit_access.uniform_and_storage_buffer16_bit_access == vk::TRUE
        && subgroup_extended_types.shader_subgroup_extended_types == vk::TRUE
        && dynamic_rendering.dynamic_rendering == vk::TRUE
        && synchronization2.synchronization2 == vk::TRUE
}

fn find_valid_queue(
    instance: &Instance,
    device: PhysicalDevice,
    surface: SurfaceKHR,
    surface_loader: &surface::Instance,
) -> Option<u32> {
    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(device) };

    let maybe_queue_family_index =
        queue_family_properties
            .iter()
            .enumerate()
            .find_map(|(index, info)| {
                let supports_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                let supports_compute = info.queue_flags.contains(vk::QueueFlags::COMPUTE);
                let supports_surface = unsafe {
                    surface_loader
                        .get_physical_device_surface_support(device, index as u32, surface)
                        .unwrap_or(false)
                };

                if supports_graphics && supports_compute && supports_surface {
                    Some(index as u32)
                } else {
                    None
                }
            });

    maybe_queue_family_index
}
