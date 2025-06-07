use alloc::sync::Arc;
use core::fmt::Debug;

pub use new::VulkanCreationError;

use ash::{ext, khr, vk};
use ash_helper::{Context, DebugUtils, VulkanContext};
use parking_lot::{Mutex, MutexGuard};
use tracing::error;

mod drop;
mod new;

/// The Vulkan Context, contains core devices for using Vulkan.
///
/// Extension devices that are relevant across different components are also created here.
/// Up to two queues are created, one for Graphics and one for Compute.
pub struct Vulkan {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,

    queue_family_index: u32,
    queues: Vec<Mutex<vk::Queue>>,

    debug_utils: Option<DebugUtils>,
    push_descriptor_device: khr::push_descriptor::Device,
    shader_object_device: ext::shader_object::Device,

    /// A command pool with the Transient Flag, used for any component to run onetime commands.
    transient_pool: Arc<Mutex<vk::CommandPool>>,
}

impl Debug for Vulkan {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let properties = unsafe {
            self.instance
                .get_physical_device_properties(self.physical_device)
        };

        let api_version = {
            let major = vk::api_version_major(properties.api_version);
            let minor = vk::api_version_minor(properties.api_version);
            let patch = vk::api_version_patch(properties.api_version);

            format!("{major}.{minor}.{patch}")
        };

        let device_name = properties.device_name_as_c_str().unwrap_or(c"Invalid name");

        f.debug_struct("Vulkan")
            .field("device_name", &device_name)
            .field("device_type", &properties.device_type)
            .field("api_version", &api_version)
            .field("queue_family_index", &self.queue_family_index)
            .field("queue_count", &self.queues.len())
            .field("debug", &self.debug_utils.is_some())
            .finish_non_exhaustive()
    }
}

/// The purpose of a created queue.
#[derive(Clone, Copy)]
#[allow(clippy::exhaustive_enums)]
pub enum QueuePurpose {
    /// The Queue is reserved for graphics (Renderer).
    Graphics,

    /// The Queue is reserved for compute (Tonemapper, ImageMaximum, etc).
    Compute,
}

impl Vulkan {
    /// Gets a reference to the mutex for the transient pool.
    /// The transient pool is a command pool with the Transient Flag, used for any component to run
    /// onetime commands.
    pub unsafe fn transient_pool(&self) -> &Mutex<vk::CommandPool> {
        &self.transient_pool
    }

    /// Clones the arc mutex for the transient pool.
    /// The transient pool is a command pool with the Transient Flag, used for any component to run
    /// onetime commands.
    pub unsafe fn clone_transient_pool(&self) -> Arc<Mutex<vk::CommandPool>> {
        Arc::clone(&self.transient_pool)
    }

    /// Returns the queue that was allocated for the given purpose
    #[inline]
    pub unsafe fn queue(&self, purpose: QueuePurpose) -> &Mutex<vk::Queue> {
        match purpose {
            QueuePurpose::Compute => self.queues.first().unwrap(),
            QueuePurpose::Graphics => self.queues.last().unwrap(),
        }
    }

    /// Waits for the device to idle.
    /// Takes and returns a lock on all queues.
    #[must_use]
    pub unsafe fn device_wait_idle(&self) -> Vec<MutexGuard<'_, vk::Queue>> {
        let locks = self.queues.iter().map(|queue| queue.lock()).collect();

        if let Err(error) = unsafe { self.device.device_wait_idle() } {
            error!("Failed to wait for device idle: {error}");
        }

        locks
    }
}

impl Context<khr::push_descriptor::Device> for Vulkan {
    #[inline]
    unsafe fn context(&self) -> &khr::push_descriptor::Device {
        &self.push_descriptor_device
    }
}

impl Context<ext::shader_object::Device> for Vulkan {
    #[inline]
    unsafe fn context(&self) -> &ext::shader_object::Device {
        &self.shader_object_device
    }
}

impl VulkanContext for Vulkan {
    #[inline]
    unsafe fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    #[inline]
    unsafe fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    #[inline]
    unsafe fn device(&self) -> &ash::Device {
        &self.device
    }

    #[inline]
    unsafe fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    #[inline]
    unsafe fn debug(&self) -> Option<&ext::debug_utils::Device> {
        if let Some(debug_utils) = self.debug_utils.as_ref() {
            Some(&debug_utils.device)
        } else {
            None
        }
    }

    #[inline]
    fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    #[inline]
    fn queue_family_index_as_slice(&self) -> &[u32] {
        core::slice::from_ref(&self.queue_family_index)
    }
}
