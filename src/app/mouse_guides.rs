use glium::Display;
use imgui::Ui;

use super::App;

impl App {
    pub fn draw_mouse_guides(&mut self, ui: &mut Ui, display: &Display) {
        let mouse_pos = ui.io().mouse_pos;
        let window_size = display.gl_window().window().inner_size();

        if mouse_pos[1] >= 0.0 && mouse_pos[1] <= window_size.height as f32 {
            ui.get_foreground_draw_list()
                .add_line(
                    [0.0, mouse_pos[1]],
                    [window_size.width as f32, mouse_pos[1]],
                    0x20_80_80_80,
                )
                .build();
        }

        if mouse_pos[0] >= 0.0 && mouse_pos[0] <= window_size.width as f32 {
            ui.get_foreground_draw_list()
                .add_line(
                    [mouse_pos[0], 0.0],
                    [mouse_pos[0], window_size.height as f32],
                    0x20_80_80_80,
                )
                .build();
        }
    }
}
