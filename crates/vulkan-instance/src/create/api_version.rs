use ash::{vk, Entry};
use tracing::{info, instrument};

use crate::VulkanError;

use super::Error;

#[instrument(skip_all, level = tracing::Level::DEBUG, err)]
pub fn validate_api_version(entry: &Entry) -> Result<(), Error> {
    let api_version = unsafe { entry.try_enumerate_instance_version() }
        .map_err(|e| VulkanError::VkResult(e, "enumerating instance version"))?
        .unwrap_or(vk::make_api_version(0, 1, 0, 0));

    let major = vk::api_version_major(api_version);
    let minor = vk::api_version_minor(api_version);
    let patch = vk::api_version_patch(api_version);

    // min-supported api version: 1.3.x
    if major != 1 || minor < 3 {
        return Err(Error::UnsupportedVulkanVersion(major, minor, patch));
    }
    info!("Vulkan API v{major}.{minor}.{patch}");

    Ok(())
}
