use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

use crate::{HdrCapture, LogicalBounds};

/// A rectangular area inside a capture to save
#[derive(Default, Debug)]
pub struct Selection {
    pub pos: PhysicalPosition<u32>,
    pub size: PhysicalSize<u32>,
}

impl From<&HdrCapture> for Selection {
    fn from(value: &HdrCapture) -> Self {
        Self {
            pos: PhysicalPosition::new(0, 0),
            size: value.size,
        }
    }
}

impl Selection {
    pub fn logcal_bounds(&self, scale_factor: f64) -> LogicalBounds {
        let logical_pos = self.pos.to_logical(scale_factor);
        let logical_size = self.size.to_logical(scale_factor);

        LogicalBounds::from((logical_pos, logical_size))
    }

    pub fn get_rect(&self, scale_factor: f64) -> [LogicalPosition<f32>; 2] {
        let pos = self.pos.to_logical(scale_factor);
        let size: LogicalSize<f32> = self.size.to_logical(scale_factor);

        [
            pos,
            LogicalPosition::new(pos.x + size.width, pos.y + size.height),
        ]
    }

    pub fn from_points(
        point_a: LogicalPosition<f32>,
        point_b: LogicalPosition<f32>,
        scale_factor: f64,
    ) -> Self {
        let bounds = LogicalBounds::from((point_a, point_b));
        let pos = LogicalPosition::new(bounds.left, bounds.top);
        let size = LogicalSize::new(bounds.right - bounds.left, bounds.bottom - bounds.top);

        Self {
            pos: pos.to_physical(scale_factor),
            size: size.to_physical(scale_factor),
        }
    }
}
