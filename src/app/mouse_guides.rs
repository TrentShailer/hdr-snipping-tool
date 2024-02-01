use glium::Display;
use imgui::Ui;

use super::App;

impl App {
    pub fn draw_mouse_guides(&mut self, ui: &mut Ui, display: &Display) {
        ui.get_foreground_draw_list()
            .add_line(
                [0.0, ui.io().mouse_pos[1]],
                [
                    display.gl_window().window().inner_size().width as f32,
                    ui.io().mouse_pos[1],
                ],
                0x20_80_80_80,
            )
            .build();

        ui.get_foreground_draw_list()
            .add_line(
                [ui.io().mouse_pos[0], 0.0],
                [
                    ui.io().mouse_pos[0],
                    display.gl_window().window().inner_size().height as f32,
                ],
                0x20_80_80_80,
            )
            .build();
    }
}
