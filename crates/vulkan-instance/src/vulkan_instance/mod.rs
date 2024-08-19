mod command_buffer;
mod debug;
mod drop;
mod fences;
mod instance;
mod logical_device;
pub(crate) mod physical_device;

use std::sync::Arc;

use ash::{
    khr::surface,
    vk::{self},
    Entry, LoadingError,
};
use command_buffer::get_command_buffer;
use debug::setup_debug;
use fences::get_fence;
use instance::aquire_instance;
use logical_device::get_logical_device;
use physical_device::get_physical_device;
use thiserror::Error;
use tracing::{info, info_span};
use winit::{
    raw_window_handle::{HandleError, HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::VulkanInstance;

impl VulkanInstance {
    pub fn new(window: Arc<Window>, debug: bool) -> Result<Self, Error> {
        let _span = info_span!("VulkanInstance::new").entered();

        // Load vk library
        let entry = unsafe { Entry::load()? };

        // api_version
        {
            let api_version = unsafe { entry.try_enumerate_instance_version() }
                .map_err(|e| Error::Vulkan(e, "enumerating instance version"))?
                .unwrap_or(vk::make_api_version(0, 1, 0, 0));

            let major = vk::api_version_major(api_version);
            let minor = vk::api_version_minor(api_version);
            let patch = vk::api_version_patch(api_version);

            // min-supported api version: 1.3.x
            if major != 1 || minor < 3 {
                return Err(Error::UnsupportedVulkanVersion(major, minor, patch));
            }
            info!("Vulkan API v{major}.{minor}.{patch}");
        }

        let instance = aquire_instance(&entry, window.clone(), debug)?;

        let (debug_utils_loader, debug_messenger) = setup_debug(&entry, &instance)?;

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )
            .map_err(|e| Error::Vulkan(e, "creating surface"))?
        };
        let surface_loader = surface::Instance::new(&entry, &instance);

        let (physical_device, queue_family_index) =
            get_physical_device(&instance, surface, &surface_loader)?;

        let (device, queue) = get_logical_device(&instance, physical_device, queue_family_index)?;
        let device = Arc::new(device);

        let (command_pool, command_buffer) =
            get_command_buffer(device.clone(), queue_family_index)?;

        let fence = get_fence(device.clone())?;

        Ok(Self {
            entry,
            instance,
            //
            device,
            physical_device,
            //
            queue,
            queue_family_index,
            //
            surface_loader,
            surface,
            //
            command_buffer_pool: command_pool,
            command_buffer,
            fence,
            //
            debug_utils_loader,
            debug_messenger,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load vulkan library:\n{0}")]
    Entry(#[from] LoadingError),

    #[error("Failed to get window/display handle:\n{0}")]
    Handle(#[from] HandleError),

    #[error("Vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("Vulkan api version {0}.{1}.{2} is unsupported, only v1.3.x is supported.")]
    UnsupportedVulkanVersion(u32, u32, u32),

    #[error("No suitable devices")]
    NoSuitableDevices,
}
