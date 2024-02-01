use imgui::Ui;

use super::App;

impl App {
    pub fn handle_selection(&mut self, ui: &mut Ui, pos: [f32; 2], size: [f32; 2]) {
        if ui.is_mouse_down(imgui::MouseButton::Left)
            && !self.selecting
            && !is_inside(ui.io().mouse_pos, pos, size)
        {
            self.selecting = true;
            self.selection_start = ui.io().mouse_pos;
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left) && self.selecting {
            let cur_pos = ui.io().mouse_pos;
            let start_pos = self.selection_start;

            // find the leftmost point
            let left = if cur_pos[0] < start_pos[0] {
                cur_pos[0]
            } else {
                start_pos[0]
            };

            let right = if cur_pos[0] > start_pos[0] {
                cur_pos[0]
            } else {
                start_pos[0]
            };

            let top = if cur_pos[1] < start_pos[1] {
                cur_pos[1]
            } else {
                start_pos[1]
            };

            let bottom = if cur_pos[1] > start_pos[1] {
                cur_pos[1]
            } else {
                start_pos[1]
            };

            self.image.selection_pos = [left as u32, top as u32];
            self.image.selection_size = [(right - left) as u32, (bottom - top) as u32];
        }

        if ui.is_mouse_released(imgui::MouseButton::Left) && self.selecting {
            self.selecting = false;
        }
    }
}

fn is_inside(point: [f32; 2], pos: [f32; 2], size: [f32; 2]) -> bool {
    point[0] >= pos[0]
        && point[0] <= pos[0] + size[0]
        && point[1] >= pos[1]
        && point[1] <= pos[1] + size[1]
}
