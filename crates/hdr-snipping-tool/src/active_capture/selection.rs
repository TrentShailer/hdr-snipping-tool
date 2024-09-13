use hdr_capture::Rect;
use winit::dpi::PhysicalPosition;

#[derive(Debug, PartialEq, Default)]
pub enum SelectionState {
    /// User has clicked but not started selecting
    Clicked(PhysicalPosition<u32>),

    /// Selection in progress
    Selecting,

    /// No currently active selection
    #[default]
    None,
}

#[derive(Debug, Default)]
pub struct Selection {
    pub rect: Rect,
    pub state: SelectionState,
}

impl Selection {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    pub fn start_selection(&mut self, position: PhysicalPosition<u32>) {
        self.state = SelectionState::Clicked(position);
    }

    pub fn end_selection(&mut self) -> bool {
        let valid_selection = self.state == SelectionState::Selecting;
        self.state = SelectionState::None;

        valid_selection
    }

    pub fn update_selection(&mut self, position: PhysicalPosition<u32>) {
        match self.state {
            SelectionState::Clicked(start) => {
                if position.x == self.rect.start[0] || position.y == self.rect.start[1] {
                    return;
                }

                self.rect.start = start.into();
                self.rect.end = position.into();
                self.state = SelectionState::Selecting;
            }

            SelectionState::Selecting => {
                if position.x != self.rect.start[0] {
                    self.rect.end[0] = position.x;
                } else {
                    self.rect.end[0] = Self::nonzero_size(self.rect.start[0], self.rect.end[0]);
                }

                if position.y != self.rect.start[1] {
                    self.rect.end[1] = position.y;
                } else {
                    self.rect.end[1] = Self::nonzero_size(self.rect.start[1], self.rect.end[1]);
                }
            }

            SelectionState::None => (),
        }
    }

    fn nonzero_size(start: u32, end: u32) -> u32 {
        if end > start {
            start + 1
        } else {
            start - 1
        }
    }
}
