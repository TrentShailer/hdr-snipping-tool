use winit::dpi::{LogicalPosition, LogicalSize};

pub struct LogicalBounds {
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
    pub top: f32,
}

impl LogicalBounds {
    pub fn contains(&self, point: &LogicalPosition<f32>) -> bool {
        point.x >= self.left
            && point.y >= self.top
            && point.x <= self.right
            && point.y <= self.bottom
    }
}

impl From<(LogicalPosition<f32>, LogicalPosition<f32>)> for LogicalBounds {
    fn from(value: (LogicalPosition<f32>, LogicalPosition<f32>)) -> Self {
        let pos_1 = value.0;
        let pos_2 = value.1;

        let bottom = pos_1.y.max(pos_2.y);
        let left = pos_1.x.min(pos_2.x);
        let right = pos_1.x.max(pos_2.x);
        let top = pos_1.y.min(pos_2.y);

        Self {
            bottom,
            left,
            right,
            top,
        }
    }
}

impl From<(LogicalPosition<f32>, LogicalSize<f32>)> for LogicalBounds {
    fn from(value: (LogicalPosition<f32>, LogicalSize<f32>)) -> Self {
        let pos = value.0;
        let size = value.1;

        Self {
            bottom: pos.y + size.height,
            left: pos.x,
            right: pos.x + size.width,
            top: pos.y,
        }
    }
}
