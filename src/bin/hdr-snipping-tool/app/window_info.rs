use glium::{glutin::error::NotSupportedError, Display};
use hdr_snipping_tool::{
    LogicalBounds, LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize,
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

impl TryFrom<&Display> for WindowInfo {
    type Error = NotSupportedError;

    fn try_from(value: &Display) -> Result<Self, Self::Error> {
        let pos = value.gl_window().window().inner_position()?;
        let size = value.gl_window().window().inner_size();

        Ok(Self {
            scale: value.gl_window().window().scale_factor(),
            pos,
            size,
        })
    }
}
