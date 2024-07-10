use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct DisplayInfo {
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
    pub maximum_luminance: f32,
    pub sdr_whitepoint: f32,
    pub sdr_whitepoint_nits: f32,
}

impl DisplayInfo {
    pub fn new(
        size: PhysicalSize<u32>,
        position: PhysicalPosition<i32>,
        maximum_luminance: f32,
        sdr_whitepoint: f32,
        sdr_whitepoint_nits: f32,
    ) -> Self {
        Self {
            size,
            position,
            maximum_luminance,
            sdr_whitepoint,
            sdr_whitepoint_nits,
        }
    }
}
