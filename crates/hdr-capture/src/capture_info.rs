use winit::dpi::{PhysicalPosition, PhysicalSize};

/// Information about a capture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaptureInfo {
    /// Physical size of the capture.
    pub size: PhysicalSize<u32>,

    /// Top left corner of the capture.<br>
    /// Relative to the top left corner of the primary display.
    pub position: PhysicalPosition<i32>,
}

impl CaptureInfo {
    pub fn new(size: PhysicalSize<u32>, position: PhysicalPosition<i32>) -> Self {
        Self { size, position }
    }
}
