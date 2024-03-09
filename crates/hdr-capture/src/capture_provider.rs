use crate::{DisplayInfo, HdrCapture};

pub trait CaptureProvider {
    type Error: std::error::Error;
    fn take_capture(&self) -> Result<(HdrCapture, DisplayInfo), Self::Error>;
}
