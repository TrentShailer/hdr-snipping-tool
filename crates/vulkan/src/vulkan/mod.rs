use alloc::sync::Arc;

pub use new::Error;

use ash::{khr, vk};
use ash_helper::{DebugUtils, VulkanContext};
use parking_lot::Mutex;

mod drop;
mod new;

/// The Vulkan Context, contains core devices for using Vulkan.
///
/// Exension devices that are relevant across different components are also created here.
/// Up to two queues are created, one for Graphics and one for Compute.
pub struct Vulkan {
    entry: ash::Entry,

    #[allow(unused)]
    vp_entry: vp_ash::Entry,
    capabilities: vp_ash::Capabilities,

    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,

    queue_family_index: u32,
    queues: Vec<Mutex<vk::Queue>>,

    debug_utils: Option<DebugUtils>,
    push_descriptor_device: khr::push_descriptor::Device,

    /// A command pool with the Transient Flag, used for any component to run onetime commands.
    transient_pool: Arc<Mutex<vk::CommandPool>>,
}

/// The purpose of a created queue.
pub enum QueuePurpose {
    /// The Queue is reserved for graphics (Renderer).
    Graphics,

    /// The Queue is reserved for compute (Tonemapper, ImageMaximum, etc).
    Compute,
}

impl Vulkan {
    /// Gets a reference to the push descriptor device.
    pub unsafe fn push_descriptor_device(&self) -> &khr::push_descriptor::Device {
        &self.push_descriptor_device
    }

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
        self.transient_pool.clone()
    }

    /// Returns the queue that was allocated for the given purpose
    #[inline]
    pub unsafe fn queue(&self, purpose: QueuePurpose) -> &Mutex<vk::Queue> {
        match purpose {
            QueuePurpose::Compute => self.queues.first().unwrap(),
            QueuePurpose::Graphics => self.queues.last().unwrap(),
        }
    }

    /// Returns the queue family index as a reference.
    #[inline]
    pub fn queue_family_index_as_ref(&self) -> &u32 {
        &self.queue_family_index
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
    unsafe fn debug(&self) -> Option<&ash::ext::debug_utils::Device> {
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
}
