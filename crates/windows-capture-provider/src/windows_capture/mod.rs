mod retrieve_handle;
mod start_capture_session;
pub mod take_capture;

use thiserror::Error;
use windows::Win32::Foundation::HANDLE;
use windows_result::Error as WindowsError;

use crate::display::Display;

/// A capture from windows in R16G16B16A16_Float format
#[derive(Debug)]
#[non_exhaustive]
pub struct WindowsCapture {
    /// Handle to the shared Dx11 texture resource.
    pub handle: HANDLE,

    /// The size of the capture.
    pub size: [u32; 2],

    /// The display the capture is of.
    pub display: Display,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to start capture session:\n{0}")]
    StartCaptureSession(#[from] start_capture_session::Error),

    #[error("Failed to retrieve capture handle:\n{0}")]
    RetrieveHandle(#[source] WindowsError),

    #[error("Failed to cleanup capture resources:\n{0}")]
    Cleanup(#[source] WindowsError),
}
