use std::sync::Arc;

use ash::{
    vk::{self, Fence},
    Device,
};

use super::Error;

pub fn get_fence(device: Arc<Device>) -> Result<Fence, Error> {
    let fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

    let fence = unsafe { device.create_fence(&fence_create_info, None) }
        .map_err(|e| Error::Vulkan(e, "creating fence"))?;

    Ok(fence)
}
