use winit::dpi::{PhysicalPosition, PhysicalSize};

/// Information about the display the capture was taken from.<br>
/// The position is the top left corner of the display relative to the top left corner of the primary display.
#[derive(Default, Debug, PartialEq)]
pub struct DisplayInfo {
    /// The top left corner of the display relative to the top left corner of the primary display
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

impl DisplayInfo {
    pub fn new(position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Self {
        Self { size, position }
    }
}
