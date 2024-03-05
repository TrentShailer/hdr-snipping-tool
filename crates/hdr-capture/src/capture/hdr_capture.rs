use winit::dpi::PhysicalSize;

/// An HDR capture using using the RGBA_f32 pixel format
#[derive(Default, Debug)]
pub struct HdrCapture {
    pub data: Box<[f32]>,
    pub size: PhysicalSize<u32>,
}

impl HdrCapture {
    pub fn new(data: Box<[f32]>, size: PhysicalSize<u32>) -> Self {
        Self { data, size }
    }
}
