use std::sync::Arc;

use vulkano::{
    device::{
        physical::PhysicalDevice, Device, DeviceCreateInfo, DeviceExtensions, Features, Queue,
        QueueCreateInfo,
    },
    Validated, VulkanError,
};

use super::requirements::{QUEUE_COUNT, REQUIRED_EXTENSIONS, REQUIRED_FEATURES};

pub fn get_logical_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    supported_optional_features: Features,
    feature_extensions: DeviceExtensions,
) -> Result<(Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>), Validated<VulkanError>> {
    let required_features = REQUIRED_FEATURES
        .map(|f| f.features)
        .into_iter()
        .reduce(|f, acc| acc | f)
        .unwrap_or(Features::empty());

    let device_create_info = DeviceCreateInfo {
        enabled_extensions: REQUIRED_EXTENSIONS | feature_extensions,
        enabled_features: required_features | supported_optional_features,
        queue_create_infos: vec![QueueCreateInfo {
            queue_family_index,
            queues: vec![0.5; QUEUE_COUNT as usize],
            ..Default::default()
        }],
        ..Default::default()
    };

    Device::new(physical_device, device_create_info)
}
