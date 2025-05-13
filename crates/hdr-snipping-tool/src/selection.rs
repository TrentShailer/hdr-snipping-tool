use winit::dpi::{PhysicalPosition, PhysicalSize};

pub trait SelectionState {
    fn handle_event(self: Box<Self>, event: SelectionEvent) -> Option<Box<dyn SelectionState>>;

    /// Has the selection been submitted.
    fn is_submitted(&self) -> bool;

    fn selection(&self) -> Option<Selection>;
}

pub enum SelectionEvent {
    MouseMoved(PhysicalPosition<f32>),
    MouseReleased,
}

#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub start: PhysicalPosition<f32>,
    pub end: PhysicalPosition<f32>,
}
impl Selection {
    pub fn mouse_clicked(position: PhysicalPosition<f32>) -> Box<dyn SelectionState> {
        Box::new(Started(position))
    }

    pub fn position(&self) -> PhysicalPosition<f32> {
        let x = self.start.x.min(self.end.x);
        let y = self.start.y.min(self.end.y);

        PhysicalPosition { x, y }
    }

    pub fn size(&self) -> PhysicalSize<f32> {
        let left = self.start.x.min(self.end.x);
        let right = self.start.x.max(self.end.x);
        let top = self.start.y.min(self.end.y);
        let bottom = self.start.y.max(self.end.y);

        PhysicalSize::new(right - left, bottom - top)
    }
}

struct Selected(Selection);
impl SelectionState for Selected {
    fn handle_event(self: Box<Self>, _event: SelectionEvent) -> Option<Box<dyn SelectionState>> {
        Some(self)
    }

    fn is_submitted(&self) -> bool {
        true
    }

    fn selection(&self) -> Option<Selection> {
        Some(self.0)
    }
}

struct Selecting(Selection);
impl SelectionState for Selecting {
    fn handle_event(mut self: Box<Self>, event: SelectionEvent) -> Option<Box<dyn SelectionState>> {
        match event {
            SelectionEvent::MouseMoved(physical_position) => {
                if physical_position.x == self.0.start.x || physical_position.y == self.0.start.y {
                    Some(self)
                } else {
                    self.0.end = physical_position;
                    Some(self)
                }
            }
            SelectionEvent::MouseReleased => Some(Box::new(Selected(self.0))),
        }
    }

    fn is_submitted(&self) -> bool {
        false
    }

    fn selection(&self) -> Option<Selection> {
        Some(self.0)
    }
}

struct Started(PhysicalPosition<f32>);
impl SelectionState for Started {
    fn handle_event(self: Box<Self>, event: SelectionEvent) -> Option<Box<dyn SelectionState>> {
        match event {
            SelectionEvent::MouseMoved(physical_position) => {
                if physical_position.x == self.0.x || physical_position.y == self.0.y {
                    Some(self)
                } else {
                    Some(Box::new(Selecting(Selection {
                        start: self.0,
                        end: physical_position,
                    })))
                }
            }
            SelectionEvent::MouseReleased => None,
        }
    }

    fn is_submitted(&self) -> bool {
        false
    }

    fn selection(&self) -> Option<Selection> {
        None
    }
}
