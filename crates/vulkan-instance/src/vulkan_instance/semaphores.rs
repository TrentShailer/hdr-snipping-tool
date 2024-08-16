use std::{collections::HashMap, sync::Arc};

use ash::{
    vk::{Semaphore, SemaphoreCreateInfo},
    Device,
};

use crate::SemaphoreUsage;

use super::Error;

pub fn get_semaphpores(device: Arc<Device>) -> Result<HashMap<SemaphoreUsage, Semaphore>, Error> {
    let semaphore_create_info = SemaphoreCreateInfo::default();

    let mut semaphores = HashMap::new();

    for value in SemaphoreUsage::VALUES {
        semaphores.insert(
            value,
            unsafe { device.create_semaphore(&semaphore_create_info, None) }
                .map_err(|e| Error::Vulkan(e, "semaphore fence"))?,
        );
    }

    Ok(semaphores)
}
