#[derive(Clone, Copy, Debug)]
/// Represents a rectangle with start and end points.
pub struct Rect {
    pub start: [u32; 2],
    pub end: [u32; 2],
}

impl Rect {
    /// The size of the selection.
    pub fn size(&self) -> [u32; 2] {
        let x_size = self.end[0].abs_diff(self.start[0]);
        let y_size = self.end[1].abs_diff(self.start[1]);

        [x_size, y_size]
    }

    /// The top left point of the selection.
    pub fn left_top(&self) -> [u32; 2] {
        let left = self.end[0].min(self.start[0]);
        let top = self.end[1].min(self.start[1]);

        [left, top]
    }
}
