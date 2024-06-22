use vulkano::device::{DeviceExtensions, Features, QueueFlags};

pub const QUEUE_FLAGS: QueueFlags = QueueFlags::union(QueueFlags::COMPUTE, QueueFlags::GRAPHICS);
pub const QUEUE_COUNT: usize = 1;

pub const DEVICE_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    ..DeviceExtensions::empty()
};

pub const FEATURES: Features = Features {
    shader_float16: true,
    storage_buffer16_bit_access: true,
    uniform_and_storage_buffer16_bit_access: true,
    shader_subgroup_extended_types: true,
    pageable_device_local_memory: true,
    ..Features::empty()
};
