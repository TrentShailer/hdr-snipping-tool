use ash::{khr::surface, vk, Entry, Instance};
use tracing::instrument;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::VulkanError;

use super::Error;

#[instrument(skip_all, level = tracing::Level::DEBUG, err)]
pub fn create_surface(
    entry: &Entry,
    instance: &Instance,
    window: &Window,
) -> Result<(vk::SurfaceKHR, surface::Instance), Error> {
    let surface = unsafe {
        ash_window::create_surface(
            entry,
            instance,
            window.display_handle()?.as_raw(),
            window.window_handle()?.as_raw(),
            None,
        )
        .map_err(|e| VulkanError::VkResult(e, "creating surface"))?
    };
    let surface_loader = surface::Instance::new(entry, instance);

    Ok((surface, surface_loader))
}
