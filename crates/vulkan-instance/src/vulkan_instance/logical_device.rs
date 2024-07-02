use std::sync::Arc;

use vulkano::{
    device::{
        physical::PhysicalDevice, Device, DeviceCreateInfo, Features, Queue, QueueCreateInfo,
    },
    Validated, VulkanError,
};

use super::requirements::{DEVICE_EXTENSIONS, QUEUE_COUNT, REQUIRED_FEATURES};

pub fn get_logical_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    supported_optional_features: Features,
) -> Result<(Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>), Validated<VulkanError>> {
    let device_create_info = DeviceCreateInfo {
        enabled_extensions: DEVICE_EXTENSIONS,
        enabled_features: REQUIRED_FEATURES | supported_optional_features,
        queue_create_infos: vec![QueueCreateInfo {
            queue_family_index,
            queues: vec![0.5; QUEUE_COUNT as usize],
            ..Default::default()
        }],
        ..Default::default()
    };

    Device::new(physical_device, device_create_info)
}
