use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    device::physical::{PhysicalDevice, PhysicalDeviceType},
    instance::Instance,
    swapchain::Surface,
    VulkanError,
};

use super::requirements::{DEVICE_EXTENSIONS, FEATURES, QUEUE_COUNT, QUEUE_FLAGS};

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "No physical devices were suitable.\nRequired Features: {:?}\nRequired Extensions: {:?}\nRequired Queue Flags: {:?}\nRequired Queue Count: {}",
        FEATURES,
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
) -> Result<(Arc<PhysicalDevice>, u32), Error> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .map_err(Error::DeviceEnumeration)?;

    let devices_with_features = physical_devices.filter(|d| {
        d.supported_features().contains(&FEATURES)
            && d.supported_extensions().contains(&DEVICE_EXTENSIONS)
    });

    let devices_with_valid_queues = devices_with_features.filter_map(|d| {
        d.queue_family_properties()
            .iter()
            .enumerate()
            .position(|(family_index, family_properties)| {
                family_properties.queue_flags.contains(QUEUE_FLAGS)
                    && family_properties.queue_count >= QUEUE_COUNT as u32
                    && d.surface_support(family_index as u32, &surface)
                        .unwrap_or(false)
            })
            .map(|family_index| (d, family_index as u32))
    });

    let best_device =
        devices_with_valid_queues.min_by_key(|(device, _)| match device.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
            _ => 5,
        });

    match best_device {
        Some(v) => Ok(v),
        None => Err(Error::NoneSuitable),
    }
}
