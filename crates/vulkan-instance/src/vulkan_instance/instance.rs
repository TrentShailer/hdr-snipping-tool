use std::{ffi, sync::Arc};

use ash::{
    ext::{self},
    vk::{self, ApplicationInfo, InstanceCreateInfo},
    Entry, Instance,
};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

use super::Error;

// -----

const APP_NAME: &ffi::CStr =
    unsafe { ffi::CStr::from_bytes_with_nul_unchecked(b"HDR-Snipping-Tool\0") };

const LAYER_NAMES: [*const ffi::c_char; 1] = unsafe {
    [ffi::CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()]
};

const INSTANCE_EXTENSIONS: [*const ffi::c_char; 2] = [
    ext::debug_utils::NAME.as_ptr(),
    ext::swapchain_colorspace::NAME.as_ptr(),
];

// -----

pub fn aquire_instance(entry: &Entry, window: Arc<Window>, debug: bool) -> Result<Instance, Error> {
    // Get extensions required to create a surface for the window
    let display_handle = window.display_handle()?;
    let mut extension_names = ash_window::enumerate_required_extensions(display_handle.as_raw())
        .expect("Unsupported platform")
        .to_vec();
    extension_names.extend_from_slice(&INSTANCE_EXTENSIONS);

    let app_info = ApplicationInfo::default()
        .application_name(APP_NAME)
        .application_version(0)
        .api_version(vk::make_api_version(0, 1, 3, 0));

    let create_info = InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names);

    let create_info = if debug {
        create_info.enabled_layer_names(&LAYER_NAMES)
    } else {
        create_info
    };

    let instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .map_err(|e| Error::Vulkan(e, "creating instance"))?
    };

    Ok(instance)
}
