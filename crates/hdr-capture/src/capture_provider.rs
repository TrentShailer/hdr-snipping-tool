use crate::{CaptureInfo, DisplayInfo};

pub trait CaptureProvider {
    type Error: std::error::Error;

    /// Get a raw capture and associated capture info.
    fn get_capture(&mut self) -> Result<(Vec<u8>, DisplayInfo, CaptureInfo), Self::Error>;
}
