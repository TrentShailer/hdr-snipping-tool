use hdr_capture::LogicalBounds;
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    error::NotSupportedError,
    window::Window,
};

#[derive(Default, Clone)]
pub struct WindowInfo {
    pub scale: f64,
    pub pos: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

impl WindowInfo {
    pub fn logical_size(&self) -> LogicalSize<f32> {
        self.size.to_logical(self.scale)
    }

    /* pub fn bounds(&self) -> [PhysicalPosition<u32>; 2] {
        [
            PhysicalPosition::new(0, 0),
            PhysicalPosition::new(self.size.width, self.size.height),
        ]
    } */

    /// Get the logical bounds of the window. <br>
    /// returns in order, top, left, bottom, right
    pub fn logical_bounds(&self) -> LogicalBounds {
        LogicalBounds::from((LogicalPosition::new(0.0, 0.0), self.logical_size()))
    }
}

impl TryFrom<&Window> for WindowInfo {
    type Error = NotSupportedError;

    fn try_from(value: &Window) -> Result<Self, Self::Error> {
        let pos = value.inner_position()?;
        let size = value.inner_size();

        Ok(Self {
            scale: value.scale_factor(),
            pos,
            size,
        })
    }
}
