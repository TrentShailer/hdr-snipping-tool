use glium::Display;
use imgui::Ui;

use super::App;

impl App {
    pub fn handle_selection(
        &mut self,
        ui: &mut Ui,
        display: &Display,
        pos: [f32; 2],
        size: [f32; 2],
    ) {
        let mouse_pos = ui.io().mouse_pos;
        let window_size = display.gl_window().window().inner_size();

        if ui.is_mouse_released(imgui::MouseButton::Left) && self.selecting {
            self.selecting = false;
            self.save_and_close();
        }

        if !is_inside(mouse_pos, [0.0, 0.0], window_size.into()) {
            return;
        }

        if ui.is_mouse_down(imgui::MouseButton::Left)
            && !self.selecting
            && !is_inside(mouse_pos, pos, size)
        {
            self.selecting = true;
            self.selection_start = mouse_pos;
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left) && self.selecting {
            let start_pos = self.selection_start;

            let left = f32::min(mouse_pos[0], start_pos[0]);
            let right = f32::max(mouse_pos[0], start_pos[0]);
            let top = f32::min(mouse_pos[1], start_pos[1]);
            let bottom = f32::max(mouse_pos[1], start_pos[1]);

            self.image.selection_pos = [left as u32, top as u32];
            self.image.selection_size = [(right - left) as u32, (bottom - top) as u32];
        }
    }
}

fn is_inside(point: [f32; 2], pos: [f32; 2], size: [f32; 2]) -> bool {
    point[0] >= pos[0]
        && point[0] <= pos[0] + size[0]
        && point[1] >= pos[1]
        && point[1] <= pos[1] + size[1]
}
