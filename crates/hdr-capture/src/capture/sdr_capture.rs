use winit::dpi::PhysicalSize;

use crate::{tonemapper::Tonemapper, HdrCapture};

/// An SDR capture using the RGBA_u8 pixel format
#[derive(Default, Debug)]
pub struct SdrCapture {
    pub data: Box<[u8]>,
    pub size: PhysicalSize<u32>,
}

impl SdrCapture {
    pub fn new(data: Box<[u8]>, size: PhysicalSize<u32>) -> Self {
        Self { data, size }
    }

    pub fn from_hdr<T>(hdr_capture: &HdrCapture, tone_mapper: &T) -> Self
    where
        T: Tonemapper + ?Sized,
    {
        tone_mapper.tonemap(hdr_capture)
    }
}
