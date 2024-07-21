use winit::dpi::{PhysicalPosition, PhysicalSize};

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
    pub state: SelectionState,
    pub start: PhysicalPosition<u32>,
    pub end: PhysicalPosition<u32>,
}

impl Selection {
    pub fn new(start: PhysicalPosition<u32>, end: PhysicalPosition<u32>) -> Self {
        Self {
            start,
            end,
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
                if position.x == start.x || position.y == start.y {
                    return;
                }

                self.start = start;
                self.end = position;
                self.state = SelectionState::Selecting;
            }

            SelectionState::Selecting => {
                if position.x != self.start.x {
                    self.end.x = position.x;
                } else {
                    self.end.x = Self::nonzero_size(self.start.x, self.end.x);
                }

                if position.y != self.start.y {
                    self.end.y = position.y;
                } else {
                    self.end.y = Self::nonzero_size(self.start.y, self.end.y);
                }
            }

            SelectionState::None => (),
        }
    }

    pub fn as_ltrb(&self) -> [u32; 4] {
        let l = self.start.x.min(self.end.x);
        let r = self.start.x.max(self.end.x);
        let t = self.start.y.min(self.end.y);
        let b = self.start.y.max(self.end.y);

        [l, t, r, b]
    }

    pub fn as_pos_size(&self) -> (PhysicalPosition<u32>, PhysicalSize<u32>) {
        let ltrb = self.as_ltrb();
        let position = PhysicalPosition::new(ltrb[0], ltrb[1]);
        let size = PhysicalSize::new(ltrb[2] - ltrb[0], ltrb[3] - ltrb[1]);

        (position, size)
    }

    fn nonzero_size(start: u32, end: u32) -> u32 {
        if end > start {
            start + 1
        } else {
            start - 1
        }
    }
}
