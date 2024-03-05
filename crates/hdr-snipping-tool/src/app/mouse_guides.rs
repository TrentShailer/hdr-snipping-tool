use imgui::Ui;
use winit::dpi::LogicalPosition;

use super::App;

impl App {
    pub fn draw_mouse_guides(&mut self, ui: &Ui) {
        let mouse_pos: LogicalPosition<f32> = ui.io().mouse_pos.into();
        let window = self.window.logical_bounds();

        ui.get_foreground_draw_list()
            .add_line(
                [0.0, mouse_pos.y],
                [window.right, mouse_pos.y],
                0x20_80_80_80,
            )
            .build();

        ui.get_foreground_draw_list()
            .add_line(
                [mouse_pos.x, 0.0],
                [mouse_pos.x, window.bottom],
                0x20_80_80_80,
            )
            .build();
    }
}
