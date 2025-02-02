use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum SelectionState {
    /// User has clicked but not started selecting
    Clicked(PhysicalPosition<f32>),

    /// Selection in progress
    Selecting,

    /// No currently active selection
    #[default]
    None,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Selection {
    pub start: PhysicalPosition<f32>,
    pub end: PhysicalPosition<f32>,
    pub state: SelectionState,
}

impl Selection {
    /// Create a new inactive selection with default start and end points.
    pub fn new(start: PhysicalPosition<f32>, end: PhysicalPosition<f32>) -> Self {
        Self {
            start,
            end,
            ..Default::default()
        }
    }

    /// Start the selection.
    pub fn start_selection(&mut self, position: PhysicalPosition<f32>) {
        self.state = SelectionState::Clicked(position);
    }

    /// Updates the selection based on the position.
    pub fn update_selection(&mut self, position: PhysicalPosition<f32>) {
        match self.state {
            SelectionState::Clicked(start) => {
                // If the selection would have zero area, ignore this update.
                if position.x == self.start.x || position.y == self.start.y {
                    return;
                }

                self.start = start;
                self.end = position;
                self.state = SelectionState::Selecting;
            }

            SelectionState::Selecting => {
                // Only update end x position if the selection would have non-zero area.
                if position.x != self.start.x {
                    self.end.x = position.x;
                }

                // Only update end y position if the selection would have non-zero area.
                if position.y != self.start.y {
                    self.end.y = position.y;
                }
            }

            SelectionState::None => {}
        }
    }

    /// Gets the top left position of the selection.
    pub fn position(&self) -> PhysicalPosition<f32> {
        let left = self.start.x.min(self.end.x);
        let top = self.start.y.min(self.end.y);

        PhysicalPosition::new(left, top)
    }

    /// Gets the size of the selection.
    pub fn size(&self) -> PhysicalSize<f32> {
        let left = self.start.x.min(self.end.x);
        let right = self.start.x.max(self.end.x);
        let top = self.start.y.min(self.end.y);
        let bottom = self.start.y.max(self.end.y);

        PhysicalSize::new(right - left, bottom - top)
    }
}
