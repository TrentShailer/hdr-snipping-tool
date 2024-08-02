use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        DeviceExtensions,
    },
    instance::Instance,
    swapchain::Surface,
    VulkanError,
};

use super::requirements::{
    FeatureSupport, QUEUE_COUNT, QUEUE_FLAGS, REQUIRED_EXTENSIONS, REQUIRED_FEATURES,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "No physical devices were suitable.\nRequired Features: {:?}\nRequired Extensions: {:?}\nRequired Queue Flags: {:?}\nRequired Queue Count: {}",
        REQUIRED_FEATURES,
        REQUIRED_EXTENSIONS,
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
) -> Result<(Arc<PhysicalDevice>, u32, DeviceExtensions), Error> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .map_err(Error::DeviceEnumeration)?;

    // find the devices that support the required extensions
    let devices = physical_devices
        .filter(|device| device.supported_extensions().contains(&REQUIRED_EXTENSIONS));

    // find the devices the support the required features
    let devices = devices.filter(|device| {
        !REQUIRED_FEATURES
            .iter()
            .any(|feature| feature.is_supported(device) == FeatureSupport::NotSupported)
    });

    // find the devices that have a valid queue, bindle that queue with the device
    let devices = devices.filter_map(|device| {
        let queue_family = device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(index, properties)| {
                properties.queue_flags.contains(QUEUE_FLAGS)
                    && properties.queue_count >= QUEUE_COUNT
                    && device
                        .surface_support(index as u32, &surface)
                        .unwrap_or(false)
            });

        queue_family.map(|index| (device, index as u32))
    });

    // Choose the best device
    let best_device = devices.min_by_key(|(device, _)| match device.properties().device_type {
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
        _ => 5,
    });

    let (best_device, queue_family_index) = match best_device {
        Some(v) => v,
        None => return Err(Error::NoneSuitable),
    };

    // find any extensions to request to get the features we want
    let feature_extensions = REQUIRED_FEATURES
        .iter()
        .map(|f| match f.is_supported(&best_device) {
            FeatureSupport::SupportedExtension => f.extensions,
            _ => DeviceExtensions::empty(),
        });

    // Reduce the list into one set of device extensions
    let feature_extensions = feature_extensions
        .reduce(|acc, e| acc | e)
        .unwrap_or(DeviceExtensions::empty());

    Ok((best_device, queue_family_index, feature_extensions))
}
