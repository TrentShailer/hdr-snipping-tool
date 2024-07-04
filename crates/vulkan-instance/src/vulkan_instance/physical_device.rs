use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        DeviceExtensions, Features,
    },
    instance::Instance,
    swapchain::Surface,
    VulkanError,
};

use super::requirements::{
    FeatureSupport, OPTIONAL_FEATURES, QUEUE_COUNT, QUEUE_FLAGS, REQUIRED_EXTENSIONS,
    REQUIRED_FEATURES,
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
) -> Result<(Arc<PhysicalDevice>, u32, Features, DeviceExtensions), Error> {
    let physical_devices = instance
        .enumerate_physical_devices()
        .map_err(Error::DeviceEnumeration)?;

    // find the devices that support the required extensions
    let devices =
        physical_devices.filter(|d| d.supported_extensions().contains(&REQUIRED_EXTENSIONS));

    // find the devices that support the required features,
    // and bundle the extensions that need to be enabled for
    // those features
    let devices = devices.filter_map(|d| {
        // Filter out devices that don't support the features we need
        if REQUIRED_FEATURES
            .iter()
            .any(|f| f.is_supported(&d) == FeatureSupport::NotSupported)
        {
            return None;
        }

        // Find the list of required extensions to get the features we need
        let feature_extensions = REQUIRED_FEATURES.iter().map(|f| match f.is_supported(&d) {
            FeatureSupport::SupportedExtension => f.extensions,
            _ => DeviceExtensions::empty(),
        });

        // Reduce the list into one set of device extensions
        let feature_extensions = feature_extensions
            .reduce(|acc, e| acc | e)
            .unwrap_or(DeviceExtensions::empty());

        Some((d, feature_extensions))
    });

    // find the devices that have the queue family properties we require and bundle
    // the queue family with it
    let devices = devices.filter_map(|(d, feature_extensions)| {
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
        queue_index.map(|family_index| (d, family_index as u32, feature_extensions))
    });

    // Select the preferred type of device
    let best_device = devices.min_by_key(|(device, _, _)| match device.properties().device_type {
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
        _ => 5,
    });

    let (best_device, queue_family_index, feature_extensions) = match best_device {
        Some(v) => v,
        None => return Err(Error::NoneSuitable),
    };

    // find the suppored optional features and
    // extract them and their required extensions,
    // then reduce the many sets of features and extensions into one set.
    let optional_features = OPTIONAL_FEATURES
        .iter()
        .filter_map(|f| {
            let support = f.is_supported(&best_device);
            let extensions = match support {
                FeatureSupport::NotSupported => return None,
                FeatureSupport::SupportedExtension => f.extensions,
                _ => DeviceExtensions::empty(),
            };

            Some((f.features, extensions))
        })
        .reduce(|f, acc| (acc.0 | f.0, acc.1 | f.1));

    // unwrap the optional value with emptpy defaults should there be no supported features
    let (optional_features, optional_feature_extensions) =
        optional_features.unwrap_or((Features::empty(), DeviceExtensions::empty()));

    let feature_extensions = feature_extensions | optional_feature_extensions;

    Ok((
        best_device,
        queue_family_index,
        optional_features,
        feature_extensions,
    ))
}
