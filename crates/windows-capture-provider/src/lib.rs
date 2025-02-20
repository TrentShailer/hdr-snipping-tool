//! # Windows Capture Provider
//! Library to take and return a handle to a screenshot of a monitor in `R16G16B16A16Float` format.
//!

pub use capture::{WindowsCapture, WindowsCaptureResources};
pub use capture_item_cache::CaptureItemCache;
pub use directx::DirectX;
pub use monitor::Monitor;
pub use result::{LabelledWinResult, WinError, WinErrorSource};
pub use send::{SendHANDLE, SendHMONITOR, SendHWND};

mod capture;
mod capture_item_cache;
mod directx;
mod monitor;
mod result;
mod send;
