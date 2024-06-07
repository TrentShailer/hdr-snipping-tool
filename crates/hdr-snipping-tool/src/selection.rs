use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Debug, PartialEq, Default)]
pub enum SelectionState {
    Clicked,
    Selecting,
    #[default]
    None,
}

#[derive(Debug, Default)]
pub struct Selection {
    pub state: SelectionState,
    pub start: PhysicalPosition<u32>,
    pub end: PhysicalPosition<u32>,
    pub preemptive_start: PhysicalPosition<u32>,
}

impl Selection {
    pub fn new(start: PhysicalPosition<u32>, end: PhysicalPosition<u32>) -> Self {
        Self {
            start,
            end,
            ..Default::default()
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

    pub fn mouse_moved(&mut self, position: PhysicalPosition<i32>, window_size: PhysicalSize<u32>) {
        // We only care if we are selecting
        if self.state == SelectionState::None {
            return;
        }

        let window_size_pos = PhysicalPosition::new(window_size.width, window_size.height);

        // clamp mouse position to window
        let position: PhysicalPosition<u32> =
            clamp_position(position, PhysicalPosition::new(0, 0), window_size_pos);

        if position == self.preemptive_start || position == self.start {
            return;
        }

        if self.state == SelectionState::Clicked {
            self.state = SelectionState::Selecting;
            self.start = self.preemptive_start;
        }

        // clamp pos to window
        self.end = position;
    }

    pub fn mouse_pressed(
        &mut self,
        position: PhysicalPosition<i32>,
        window_size: PhysicalSize<u32>,
    ) {
        // Ensure mouse is in bounds
        if position.x < 0
            || position.y < 0
            || position.x > window_size.width as i32
            || position.y > window_size.height as i32
        {
            return;
        }

        if self.state == SelectionState::None {
            self.state = SelectionState::Clicked;
        }

        self.preemptive_start = position.cast();
    }

    pub fn mouse_released(&mut self) -> bool {
        let should_save = self.state == SelectionState::Selecting;
        self.state = SelectionState::None;

        should_save
    }
}

fn clamp_position(
    position: PhysicalPosition<i32>,
    min: PhysicalPosition<u32>,
    max: PhysicalPosition<u32>,
) -> PhysicalPosition<u32> {
    let min: PhysicalPosition<i32> = min.cast();
    let max: PhysicalPosition<i32> = max.cast();

    PhysicalPosition::new(
        position.x.clamp(min.x, max.x) as u32,
        position.y.clamp(min.y, max.y) as u32,
    )
}
