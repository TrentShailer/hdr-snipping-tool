use crate::{HdrCapture, SdrCapture};

pub trait Tonemapper {
    /// Performs the tonemapping on and hdr capture to produce an sdr capture
    fn tonemap(&self, hdr_capture: &HdrCapture) -> SdrCapture;
    /// Resets the settings of the tonemapper based on an hdr capture
    fn reset_settings(&mut self, hdr_capture: &HdrCapture);
}
