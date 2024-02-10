use crate::{DisplayInfo, HdrCapture};

#[cfg(windows)]
pub mod windows_capture;

pub trait CaptureProvider {
    type Error: std::error::Error;
    fn take_capture(&self) -> Result<(HdrCapture, DisplayInfo), Self::Error>;
}
