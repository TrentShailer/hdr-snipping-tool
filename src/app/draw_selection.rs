use glium::{glutin::dpi::LogicalSize, Display};
use imgui::Ui;

use super::App;

impl App {
    pub fn draw_selection(&mut self, ui: &mut Ui, display: &Display) {
        // draw border
        let selection_rect = self
            .image
            .get_selection_rect(display.gl_window().window().scale_factor());

        let top_left = selection_rect[0];
        let bottom_right = selection_rect[1];

        ui.get_background_draw_list()
            .add_rect(
                [top_left.x, top_left.y],
                [bottom_right.x, bottom_right.y],
                0xff_fc_fa_f7,
            )
            .thickness(2.0)
            .rounding(1.0)
            .build();

        // dim outside of selection
        let window: LogicalSize<f32> = display
            .gl_window()
            .window()
            .inner_size()
            .to_logical(display.gl_window().window().scale_factor());

        // top
        ui.get_background_draw_list()
            .add_rect([0.0, 0.0], [window.width, top_left.y], 0x80_00_00_00)
            .filled(true)
            .thickness(0.0)
            .build();

        // left
        ui.get_background_draw_list()
            .add_rect(
                [0.0, top_left.y],
                [top_left.x, bottom_right.y],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // bottom
        ui.get_background_draw_list()
            .add_rect(
                [0.0, bottom_right.y],
                [window.width, window.height],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // right
        ui.get_background_draw_list()
            .add_rect(
                [bottom_right.x, top_left.y],
                [window.width, bottom_right.y],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();
    }
}
