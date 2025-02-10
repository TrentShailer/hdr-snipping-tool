use core::slice;

use alloc::sync::Arc;

use ash::{khr, vk};
use ash_helper::{try_name, vulkan_debug_callback, DebugUtils, VkError};
use parking_lot::Mutex;
use raw_window_handle::RawDisplayHandle;
use thiserror::Error;
use tracing::info;
use vp_ash::vp;

use super::{QueuePurpose, Vulkan};

impl Vulkan {
    /// Creates a new instance of the Vulkan Context using the Vulkan Profiles API. Designed to
    /// support Vulkan 1.2 with the required extensions.
    pub unsafe fn new(
        try_debug: bool,
        display_handle: Option<RawDisplayHandle>,
    ) -> Result<Self, Error> {
        // Setup objects.
        let entry = ash::Entry::linked();
        let vp_entry = vp_ash::Entry::linked();

        let capabilities = {
            let create_info = vp::CapabilitiesCreateInfo::default()
                .api_version(vk::make_api_version(0, 1, 2, 198))
                .flags(vp::CapabilitiesCreateFlags::STATIC);

            unsafe { vp_entry.create_capabilities(&create_info, None) }
                .map_err(|e| VkError::new(e, "vpCreateCapabilities"))?
        };

        // Profiles for this application.
        let core_profile = vp::ProfileProperties::default()
            .profile_name(c"VP_HDR_SNIPPING_TOOL_requirements")
            .unwrap()
            .spec_version(2);
        let debug_profile = vp::ProfileProperties::default()
            .profile_name(c"VP_HDR_SNIPPING_TOOL_requirements_debug")
            .unwrap()
            .spec_version(1);

        // Sanity check that profiles are present, if the instance in the build environment is
        // missing the required extensions, this will fail.
        {
            let profiles = unsafe { capabilities.get_profiles() }
                .map_err(|e| VkError::new(e, "vpGetProfiles"))?;

            assert!(
                profiles.contains(&core_profile),
                "The build environment does not support the profiles."
            );
            assert!(
                profiles.contains(&debug_profile),
                "The build environment does not support the profiles."
            );
        };

        // Check for instance support.
        let supports_instance =
            unsafe { capabilities.get_instance_profile_support(None, &core_profile) }
                .map_err(|e| VkError::new(e, "vpGetInstanceProfileSupport"))?;
        if !supports_instance {
            return Err(Error::UnsupportedInstance);
        }

        // If the instance supports debug and debug is wanted, then we should debug.
        let should_debug = {
            let supports_debug =
                unsafe { capabilities.get_instance_profile_support(None, &debug_profile) }
                    .map_err(|e| VkError::new(e, "vpGetInstanceProfileSupport"))?;

            try_debug && supports_debug
        };

        // Create the list of profiles to use.
        let mut enabled_profiles = vec![core_profile];
        if should_debug {
            enabled_profiles.push(debug_profile);
        }

        // Create instance.
        let instance = {
            let api_version = unsafe { capabilities.get_profile_api_version(&core_profile) };

            let app_info = vk::ApplicationInfo::default()
                .api_version(api_version)
                .application_name(c"HDR Snipping Tool");

            let mut additional_extensions = vec![];

            if let Some(handle) = display_handle {
                let extensions = ash_window::enumerate_required_extensions(handle)
                    .map_err(|e| VkError::new(e, "enumerateWindowExtensions"))?;
                additional_extensions.extend_from_slice(extensions);
            }

            let vk_create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&additional_extensions);

            let vp_create_info = vp::InstanceCreateInfo::default()
                .create_info(&vk_create_info)
                .enabled_full_profiles(&enabled_profiles);

            unsafe { capabilities.create_instance(&entry, &vp_create_info, None) }
                .map_err(|e| VkError::new(e, "vpCreateInstance"))?
        };

        // Select a physical device.
        let physical_device = {
            unsafe { instance.enumerate_physical_devices() }
                .map_err(|e| VkError::new(e, "vkEnumeratePhysicalDevices"))?
                .into_iter()
                .filter_map(|device| unsafe {
                    capabilities
                        .get_physical_device_profile_support(&instance, device, &core_profile)
                        .map(|supported| if supported { Some(device) } else { None })
                        .map_err(|e| VkError::new(e, "vpGetPhysicalDeviceProfileSupport"))
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .min_by_key(|&device| {
                    let properties = unsafe { instance.get_physical_device_properties(device) };

                    match properties.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                        vk::PhysicalDeviceType::CPU => 3,
                        vk::PhysicalDeviceType::OTHER => 4,
                        _ => 5,
                    }
                })
                .ok_or(Error::UnsupportedDevice)?
        };

        // Get the queue family index and queue count.
        let (queue_family_index, queue_count) = {
            let requirements = {
                let mut count = 1;
                let mut requirements = vk::QueueFamilyProperties2KHR::default();
                unsafe {
                    capabilities
                        .get_profile_queue_family_properties(
                            &core_profile,
                            None,
                            &mut count,
                            Some(slice::from_mut(&mut requirements)),
                        )
                        .map_err(|e| VkError::new(e, "vpGetProfileQueueFamilyProperties"))?
                }

                requirements.queue_family_properties
            };

            let queue_properties =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

            let (index, properties) = queue_properties
                .into_iter()
                .enumerate()
                .find(|(_, properties)| {
                    properties.queue_count >= requirements.queue_count
                        && properties.queue_flags.contains(requirements.queue_flags)
                })
                .map(|(index, properties)| (index as u32, properties))
                .unwrap();

            let queue_count = properties.queue_count.min(2);
            (index, queue_count)
        };

        // Create logical device.
        let device = {
            let mut additional_extensions = vec![];

            // Request portability if the device supports it.
            {
                let extensions =
                    unsafe { instance.enumerate_device_extension_properties(physical_device) }
                        .map_err(|e| VkError::new(e, "vkEnumerateDeviceExtensionProperties"))?;

                let supports_portability = extensions.into_iter().any(|properties| {
                    properties.extension_name_as_c_str().unwrap_or(c"")
                        == khr::portability_subset::NAME
                });

                if supports_portability {
                    additional_extensions.push(khr::portability_subset::NAME.as_ptr());
                }
            }

            let queue_priorities = [1.0].repeat(queue_count as usize);
            let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&queue_priorities)];

            let vk_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&additional_extensions);

            let vp_create_info = vp::DeviceCreateInfo::default()
                .create_info(&vk_create_info)
                .enabled_full_profiles(&enabled_profiles);

            unsafe { capabilities.create_device(&instance, physical_device, &vp_create_info, None) }
                .map_err(|e| VkError::new(e, "vpCreateDevice"))?
        };

        // Retrieve the queues.
        let queues = (0..queue_count)
            .map(|index| Mutex::new(unsafe { device.get_device_queue(queue_family_index, index) }))
            .collect();

        // Create the push descriptor device
        let push_descriptor_device = khr::push_descriptor::Device::new(&instance, &device);

        // Create debug utils if we should debug
        let debug_utils = if should_debug {
            Some(unsafe {
                DebugUtils::new(&entry, &instance, &device, Some(vulkan_debug_callback))?
            })
        } else {
            None
        };

        // Create transient pool
        let transient_pool = {
            let create_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(queue_family_index);

            Arc::new(Mutex::new(
                unsafe { device.create_command_pool(&create_info, None) }
                    .map_err(|e| VkError::new(e, "vkCreateCommandPool"))?,
            ))
        };

        let vulkan = Self {
            entry,
            vp_entry,
            capabilities,
            instance,
            physical_device,
            device,
            queue_family_index,
            queues,
            debug_utils,
            push_descriptor_device,
            transient_pool,
        };

        info!("Created Vulkan Context: {:?}", vulkan);

        // Name objects
        unsafe {
            if queue_count == 1 {
                try_name(
                    &vulkan,
                    *vulkan.queue(QueuePurpose::Compute).lock(),
                    "Compute + Graphics Queue",
                );
            } else {
                try_name(
                    &vulkan,
                    *vulkan.queue(QueuePurpose::Compute).lock(),
                    "Compute Queue",
                );
                try_name(
                    &vulkan,
                    *vulkan.queue(QueuePurpose::Graphics).lock(),
                    "Graphics Queue",
                );
            };

            try_name(&vulkan, vulkan.device.handle(), "Main Device");
            try_name(
                &vulkan,
                vulkan.push_descriptor_device.device(),
                "Push Descriptor Device",
            );
            try_name(&vulkan, *vulkan.transient_pool().lock(), "Transient Pool");
        };

        Ok(vulkan)
    }
}

/// Error variants from trying to create the Vulkan Context.
#[derive(Debug, Error)]
pub enum Error {
    /// A Vulkan call returned an error.
    #[allow(clippy::enum_variant_names)]
    #[error(transparent)]
    VkError(#[from] VkError),

    /// The instance does not meet the requirements to use the application.
    #[error("Vulkan Instance does not meet the requirements.")]
    UnsupportedInstance,

    /// No Physical Devices meet the requirements to use the application.
    #[error("No Physical Devices meet the requirements.")]
    UnsupportedDevice,
}
