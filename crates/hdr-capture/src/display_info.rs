use winit::dpi::{PhysicalPosition, PhysicalSize};

pub struct DisplayInfo {
    pub size: PhysicalSize<u32>,
    pub position: PhysicalPosition<i32>,
}

impl DisplayInfo {
    pub fn new(size: PhysicalSize<u32>, position: PhysicalPosition<i32>) -> Self {
        Self { size, position }
    }
}
