use std::fmt::Debug;

use vulkano::{
    device::{physical::PhysicalDevice, DeviceExtensions, Features, QueueFlags},
    Version,
};

pub const QUEUE_FLAGS: QueueFlags = QueueFlags::union(QueueFlags::COMPUTE, QueueFlags::GRAPHICS);
pub const QUEUE_COUNT: u32 = 1;

pub const REQUIRED_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    ..DeviceExtensions::empty()
};

pub const REQUIRED_FEATURES: [FeatureSet; 4] = [
    FeatureSet {
        features: Features {
            shader_float16: true,
            ..Features::empty()
        },
        version: Some(Version::V1_2),
        extensions: DeviceExtensions {
            khr_shader_float16_int8: true,
            ..DeviceExtensions::empty()
        },
    },
    FeatureSet {
        features: Features {
            storage_buffer16_bit_access: true,
            uniform_and_storage_buffer16_bit_access: true,
            storage_push_constant16: true,
            ..Features::empty()
        },
        version: Some(Version::V1_1),
        extensions: DeviceExtensions {
            khr_16bit_storage: true,
            ..DeviceExtensions::empty()
        },
    },
    FeatureSet {
        features: Features {
            shader_subgroup_extended_types: true,
            ..Features::empty()
        },
        version: Some(Version::V1_2),
        extensions: DeviceExtensions {
            khr_shader_subgroup_extended_types: true,
            ..DeviceExtensions::empty()
        },
    },
    FeatureSet {
        features: Features {
            dynamic_rendering: true,
            ..Features::empty()
        },
        version: Some(Version::V1_3),
        extensions: DeviceExtensions {
            khr_dynamic_rendering: true,
            ..DeviceExtensions::empty()
        },
    },
];

pub const OPTIONAL_FEATURES: [FeatureSet; 2] = [
    FeatureSet {
        features: Features {
            pageable_device_local_memory: true,
            ..Features::empty()
        },
        version: None,
        extensions: DeviceExtensions {
            ext_pageable_device_local_memory: true,
            ..DeviceExtensions::empty()
        },
    },
    FeatureSet {
        features: Features {
            host_query_reset: true,
            ..Features::empty()
        },
        version: Some(Version::V1_2),
        extensions: DeviceExtensions {
            ext_host_query_reset: true,
            ..DeviceExtensions::empty()
        },
    },
];

/// A feature or set of features and their associated required version or extension.
pub struct FeatureSet {
    /// Feature or set of features that are desired.
    ///
    /// Sets of features must all be related to the same extension and version.
    pub features: Features,

    /// The minimum version of vulkan where the feature does not require the extension.
    pub version: Option<Version>,

    /// The extension(s) that is required to be requested if the device is below the version.
    pub extensions: DeviceExtensions,
}

/// If the feature(s) are supported, and in what way.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FeatureSupport {
    /// The feature(s) are supported without version or extension.
    Supported,
    /// The feature(s) are supported by version.
    SupportedVersion,
    /// The feature(s) aren't supported by version, but are by extension.
    SupportedExtension,
    /// The feature(s) aren't supported.
    NotSupported,
}

impl FeatureSet {
    /// Returns if and how the feature set is supported by a given device.
    pub fn is_supported(&self, device: &PhysicalDevice) -> FeatureSupport {
        if !device.supported_features().contains(&self.features) {
            return FeatureSupport::NotSupported;
        }

        if let Some(version) = self.version {
            if device.api_version() >= version {
                return FeatureSupport::SupportedVersion;
            }
        }

        if device.supported_extensions().contains(&self.extensions) {
            return FeatureSupport::SupportedExtension;
        }

        return FeatureSupport::Supported;
    }
}

impl Debug for FeatureSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeatureSet")
            .field("features", &self.features)
            .finish()
    }
}
