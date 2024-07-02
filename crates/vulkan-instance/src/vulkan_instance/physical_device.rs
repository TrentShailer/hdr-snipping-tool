use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Features,
    },
    instance::Instance,
    swapchain::Surface,
    VulkanError,
};

use super::requirements::{
    DEVICE_EXTENSIONS, OPTIONAL_FEATURES, QUEUE_COUNT, QUEUE_FLAGS, REQUIRED_FEATURES,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "No physical devices were suitable.\nRequired Features: {:?}\nRequired Extensions: {:?}\nRequired Queue Flags: {:?}\nRequired Queue Count: {}",
        REQUIRED_FEATURES,
        DEVICE_EXTENSIONS,
        QUEUE_FLAGS,
        QUEUE_COUNT
    )]
    NoneSuitable,

    #[error("Failed to enumerate devices:\n{0}")]
    DeviceEnumeration(#[source] VulkanError),
}

pub fn get_physical_device(
    instance: Arc<Instance>,
    surface: Arc<Surface>,
) -> Result<(Arc<PhysicalDevice>, u32, Features), Error> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .map_err(Error::DeviceEnumeration)?;

    // Filter deviecs to find the ones that support the features and extensions that are required
    let valid_devices = physical_devices.filter(|d| {
        d.supported_features().contains(&REQUIRED_FEATURES)
            && d.supported_extensions().contains(&DEVICE_EXTENSIONS)
    });

    let valid_devices = valid_devices.filter_map(|d| {
        // Find the index of the first queue family that meets the requirements
        let queue_index = d.queue_family_properties().iter().enumerate().position(
            |(family_index, family_properties)| {
                family_properties.queue_flags.contains(QUEUE_FLAGS)
                    && family_properties.queue_count >= QUEUE_COUNT
                    && d.surface_support(family_index as u32, &surface)
                        .unwrap_or(false)
            },
        );

        // bundle the queue family with the device
        queue_index.map(|family_index| (d, family_index as u32))
    });

    // Select the preferred type of device
    let best_device =
        valid_devices.min_by_key(|(device, _)| match device.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        });

    let best_device = match best_device {
        Some(v) => Ok(v),
        None => Err(Error::NoneSuitable),
    };

    // bundle the supported optional features with the device
    best_device.map(|(device, family_index)| {
        let supported_optionals = device.supported_features().intersection(&OPTIONAL_FEATURES);
        (device, family_index, supported_optionals)
    })
}
