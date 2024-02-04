use glium::{glutin::dpi::LogicalSize, Display};
use imgui::Ui;

use super::App;

impl App {
    pub fn draw_mouse_guides(&mut self, ui: &mut Ui, display: &Display) {
        let mouse_pos = ui.io().mouse_pos;
        let window_size: LogicalSize<f32> = display
            .gl_window()
            .window()
            .inner_size()
            .to_logical(display.gl_window().window().scale_factor());

        if mouse_pos[1] >= 0.0 && mouse_pos[1] <= window_size.height {
            ui.get_foreground_draw_list()
                .add_line(
                    [0.0, mouse_pos[1]],
                    [window_size.width, mouse_pos[1]],
                    0x20_80_80_80,
                )
                .build();
        }

        if mouse_pos[0] >= 0.0 && mouse_pos[0] <= window_size.width {
            ui.get_foreground_draw_list()
                .add_line(
                    [mouse_pos[0], 0.0],
                    [mouse_pos[0], window_size.height],
                    0x20_80_80_80,
                )
                .build();
        }
    }
}
