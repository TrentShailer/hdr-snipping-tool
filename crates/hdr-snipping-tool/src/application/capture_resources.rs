use vulkan::HdrImage;
use windows_capture_provider::{Monitor, WindowsCapture};

use crate::{capture_taker::Whitepoint, selection::Selection};

pub struct CaptureResources {
    #[expect(unused)]
    pub monitor: Monitor,
    pub capture: WindowsCapture,
    pub hdr_capture: HdrImage,
    pub whitepoint: Whitepoint,
    pub selection: Selection,
}
