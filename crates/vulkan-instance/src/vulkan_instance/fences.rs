use std::{collections::HashMap, sync::Arc};

use ash::{
    vk::{self, Fence},
    Device,
};

use crate::CommandBufferUsage;

use super::Error;

pub fn get_fences(device: Arc<Device>) -> Result<HashMap<CommandBufferUsage, Fence>, Error> {
    let fence_create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

    let mut fences = HashMap::new();

    for value in CommandBufferUsage::VALUES {
        fences.insert(
            value,
            unsafe { device.create_fence(&fence_create_info, None) }
                .map_err(|e| Error::Vulkan(e, "creating fence"))?,
        );
    }

    Ok(fences)
}
