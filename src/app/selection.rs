use glium::{
    glutin::dpi::{LogicalPosition, LogicalSize},
    Display,
};
use imgui::{Textures, Ui};
use imgui_glium_renderer::Texture;

use super::App;

#[derive(PartialEq)]
pub enum SelectionSate {
    /// User is not currenty selecting
    None,
    /// User has clicked down but not dragged
    StartedSelecting,
    /// User had clicked and dragged
    Selecting,
}

impl App {
    pub fn handle_selection(
        &mut self,
        ui: &mut Ui,
        display: &Display,
        textures: &mut Textures<Texture>,
        pos: [f32; 2],
        size: [f32; 2],
    ) {
        let mouse_pos = LogicalPosition::new(ui.io().mouse_pos[0], ui.io().mouse_pos[1]);
        let scale = display.gl_window().window().scale_factor();
        let window_size: LogicalSize<f32> =
            display.gl_window().window().inner_size().to_logical(scale);

        if ui.is_mouse_released(imgui::MouseButton::Left)
            && self.selection_state != SelectionSate::None
        {
            // prevent registering clicks as area selection
            if self.selection_state == SelectionSate::Selecting {
                self.save_and_close(display, textures);
            }

            self.selection_state = SelectionSate::None;
        }

        if !is_inside([mouse_pos.x, mouse_pos.y], [0.0, 0.0], window_size.into()) {
            return;
        }

        if ui.is_mouse_down(imgui::MouseButton::Left)
            && self.selection_state == SelectionSate::None
            && !is_inside([mouse_pos.x, mouse_pos.y], pos, size)
        {
            self.selection_state = SelectionSate::StartedSelecting;
            self.selection_start = mouse_pos;
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left)
            && self.selection_state != SelectionSate::None
        {
            self.selection_state = SelectionSate::Selecting;
            let start_pos = self.selection_start;

            let left = f32::min(mouse_pos.x, start_pos.x);
            let right = f32::max(mouse_pos.x, start_pos.x);
            let top = f32::min(mouse_pos.y, start_pos.y);
            let bottom = f32::max(mouse_pos.y, start_pos.y);

            let pos = LogicalPosition::new(left, top);
            let size = LogicalSize::new(right - left, bottom - top);

            self.image.selection_pos = pos.to_physical(scale);
            self.image.selection_size = size.to_physical(scale);
        }
    }
}

fn is_inside(point: [f32; 2], pos: [f32; 2], size: [f32; 2]) -> bool {
    point[0] >= pos[0]
        && point[0] <= pos[0] + size[0]
        && point[1] >= pos[1]
        && point[1] <= pos[1] + size[1]
}
