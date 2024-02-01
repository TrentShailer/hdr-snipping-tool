use glium::Display;
use imgui::Ui;

use super::App;

impl App {
    pub fn draw_selection(&mut self, ui: &mut Ui, display: &Display) {
        // draw border
        let selection_rect = self.image.get_selection_rect();
        ui.get_background_draw_list()
            .add_rect(selection_rect[0], selection_rect[1], 0xff_fc_fa_f7)
            .thickness(2.0)
            .rounding(1.0)
            .build();

        // dim outside of selection
        let window = display.gl_window().window().inner_size();

        // top
        ui.get_background_draw_list()
            .add_rect(
                [0.0, 0.0],
                [window.width as f32, selection_rect[0][1]],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // left
        ui.get_background_draw_list()
            .add_rect(
                [0.0, selection_rect[0][1]],
                [selection_rect[0][0], selection_rect[1][1]],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // bottom
        ui.get_background_draw_list()
            .add_rect(
                [0.0, selection_rect[1][1]],
                [window.width as f32, window.height as f32],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // right
        ui.get_background_draw_list()
            .add_rect(
                [selection_rect[1][0], selection_rect[0][1]],
                [window.width as f32, selection_rect[1][1]],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();
    }
}
