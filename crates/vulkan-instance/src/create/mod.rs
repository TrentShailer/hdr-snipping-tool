mod api_version;
mod command_buffer;
mod debug;
mod drop;
mod instance;
mod logical_device;
mod physical_device;
mod surface;

use api_version::validate_api_version;
use ash::{
    vk::{CommandBufferAllocateInfo, FenceCreateFlags, FenceCreateInfo},
    Entry, LoadingError,
};
use command_buffer::get_command_buffer;
use debug::setup_debug;
use instance::aquire_instance;
use logical_device::get_logical_device;
use physical_device::get_physical_device;
use surface::create_surface;
use thiserror::Error;
use tracing::instrument;
use winit::{raw_window_handle::HandleError, window::Window};

use crate::{VulkanError, VulkanInstance};

impl VulkanInstance {
    #[instrument("VulkanInstance::new", skip_all, err)]
    pub fn new(window: &Window, enable_validation_layers: bool) -> Result<Self, Error> {
        // Load vk library
        let entry = unsafe { Entry::load()? };

        validate_api_version(&entry)?;

        let instance = aquire_instance(&entry, window, enable_validation_layers)?;

        let debug_utils = if enable_validation_layers {
            Some(setup_debug(&entry, &instance)?)
        } else {
            None
        };

        let (surface, surface_loader) = create_surface(&entry, &instance, window)?;

        let (physical_device, queue_family_index) =
            get_physical_device(&instance, surface, &surface_loader)?;

        let (device, queue) = get_logical_device(&instance, physical_device, queue_family_index)?;
        let (command_buffer_pool, command_buffer) =
            get_command_buffer(&device, queue_family_index)?;

        // Create wake command buffer
        let wake_command_buffer = unsafe {
            let allocate_info = CommandBufferAllocateInfo::default()
                .command_buffer_count(1)
                .command_pool(command_buffer_pool);

            let command_buffer = device
                .allocate_command_buffers(&allocate_info)
                .map_err(|e| VulkanError::VkResult(e, "allocating command buffers"))?[0];

            let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);

            let fence = device
                .create_fence(&fence_create_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating fence"))?;

            (command_buffer, fence)
        };

        Ok(Self {
            entry,
            instance,

            physical_device,
            device,
            queue,
            queue_family_index,

            surface_loader,
            surface,

            command_buffer_pool,
            command_buffer,
            wake_command_buffer,

            debug_utils,
        })
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to load vulkan library:\n{0}")]
    LoadLibrary(#[from] LoadingError),

    #[error("Failed to get window/display handle:\n{0}")]
    GetHandle(#[from] HandleError),

    #[error(transparent)]
    GenericVulkan(#[from] VulkanError),

    #[error("Vulkan api version {0}.{1}.{2} is unsupported, only v1.3.x is supported.")]
    UnsupportedVulkanVersion(u32, u32, u32),

    #[error("No suitable devices")]
    NoSuitableDevices,
}
