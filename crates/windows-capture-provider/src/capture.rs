use windows::Win32::Foundation::HANDLE;

use crate::display::Display;

/// A capture and it's metadata.
pub struct Capture {
    /// Handle to the shared Dx11 texture resource.
    pub handle: HANDLE,

    /// The display the capture is of.
    pub display: Display,
}
