use std::sync::Arc;

use vulkano::{
    device::{physical::PhysicalDevice, Device, DeviceCreateInfo, Queue, QueueCreateInfo},
    Validated, VulkanError,
};

use super::requirements::{DEVICE_EXTENSIONS, FEATURES, QUEUE_COUNT};

pub fn get_logical_device(
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
) -> Result<(Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>), Validated<VulkanError>> {
    let device_create_info = DeviceCreateInfo {
        enabled_extensions: DEVICE_EXTENSIONS,
        enabled_features: FEATURES,
        queue_create_infos: vec![QueueCreateInfo {
            queue_family_index,
            queues: vec![0.5; QUEUE_COUNT],
            ..Default::default()
        }],
        ..Default::default()
    };

    Device::new(physical_device, device_create_info)
}
