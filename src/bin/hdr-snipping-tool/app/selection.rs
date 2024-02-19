use hdr_snipping_tool::{LogicalBounds, LogicalPosition, Selection};
use imgui::Ui;

use super::{app_event::AppEvent, App};

#[derive(PartialEq, Default)]
pub enum SelectionSate {
    /// User is not currenty selecting
    #[default]
    None,
    /// User has clicked down but not dragged
    StartedSelecting,
    /// User had clicked and dragged
    Selecting,
}

impl App {
    pub fn draw_selection(&self, ui: &Ui) {
        // draw border
        let selection = self.capture.selection.logcal_bounds(self.window.scale);

        ui.get_background_draw_list()
            .add_rect(
                [selection.left, selection.top],
                [selection.right, selection.bottom],
                0xff_fc_fa_f7,
            )
            .thickness(2.0)
            .rounding(1.0)
            .build();

        // dim outside of selection
        let window = self.window.logical_bounds();

        // top
        ui.get_background_draw_list()
            .add_rect(
                [window.left, window.top],
                [window.right, selection.top],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // left
        ui.get_background_draw_list()
            .add_rect(
                [window.left, selection.top],
                [selection.left, selection.bottom],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // bottom
        ui.get_background_draw_list()
            .add_rect(
                [window.left, selection.bottom],
                [window.right, window.bottom],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();

        // right
        ui.get_background_draw_list()
            .add_rect(
                [selection.right, selection.top],
                [window.right, selection.bottom],
                0x80_00_00_00,
            )
            .filled(true)
            .thickness(0.0)
            .build();
    }

    pub fn handle_selection(&mut self, ui: &Ui, settings_bounds: LogicalBounds) {
        let mouse_pos: LogicalPosition<f32> = ui.io().mouse_pos.into();
        let window = self.window.logical_bounds();

        if ui.is_mouse_released(imgui::MouseButton::Left)
            && self.selection_state != SelectionSate::None
        {
            // prevent registering clicks as area selection
            if self.selection_state == SelectionSate::Selecting {
                self.event_queue
                    .append(&mut [AppEvent::Save, AppEvent::Close].into());
            }

            self.selection_state = SelectionSate::None;
            return;
        }

        if !window.contains(&mouse_pos) {
            return;
        }

        if ui.is_mouse_down(imgui::MouseButton::Left)
            && self.selection_state == SelectionSate::None
            && !settings_bounds.contains(&mouse_pos)
        {
            self.selection_state = SelectionSate::StartedSelecting;
            self.selection_start = mouse_pos;
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left)
            && self.selection_state != SelectionSate::None
        {
            self.selection_state = SelectionSate::Selecting;

            self.capture.selection =
                Selection::from_points(self.selection_start, mouse_pos, self.window.scale);
        }
    }
}