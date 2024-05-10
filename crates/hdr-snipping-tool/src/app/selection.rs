use error_trace::{ErrorTrace, ResultExt};
use hdr_capture::{LogicalBounds, Selection, Tonemapper};
use imgui::{MouseButton, Ui};
use winit::dpi::LogicalPosition;

use super::{settings::ImguiSettings, App};

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

#[derive(PartialEq, Debug)]
enum MouseState {
    Clicked,
    Released,
    Dragging,
    None,
}

impl<T: Tonemapper + ImguiSettings> App<T> {
    pub fn draw_selection(&self, ui: &Ui) {
        let window = self.window.logical_bounds();
        let selection = self.capture.selection.logcal_bounds(self.window.scale);

        // Draw selection outline
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

    fn get_mouse_state(ui: &Ui, mouse_button: MouseButton) -> MouseState {
        if ui.is_mouse_released(mouse_button) {
            MouseState::Released
        } else if ui.is_mouse_dragging(mouse_button) {
            MouseState::Dragging
        } else if ui.is_mouse_clicked(mouse_button) {
            MouseState::Clicked
        } else {
            MouseState::None
        }
    }

    fn can_start_selecting(
        &self,
        mouse_pos: &LogicalPosition<f32>,
        settings_bounds: &LogicalBounds,
        window_bounds: &LogicalBounds,
    ) -> bool {
        if self.selection_state != SelectionSate::None {
            return false;
        }
        if settings_bounds.contains(mouse_pos) {
            return false;
        }
        if !window_bounds.contains(mouse_pos) {
            return false;
        }

        true
    }

    pub fn handle_selection(
        &mut self,
        ui: &Ui,
        settings_bounds: LogicalBounds,
    ) -> Result<(), ErrorTrace> {
        let mouse_pos: LogicalPosition<f32> = ui.io().mouse_pos.into();
        let window = self.window.logical_bounds();
        let mouse_state = Self::get_mouse_state(ui, MouseButton::Left);

        match mouse_state {
            MouseState::Clicked => {
                if self.can_start_selecting(&mouse_pos, &settings_bounds, &window) {
                    self.selection_state = SelectionSate::StartedSelecting;
                    self.selection_start = mouse_pos;
                }
            }
            MouseState::Dragging => {
                if self.selection_state != SelectionSate::None {
                    // clamp mouse pos to window
                    let mouse_pos = LogicalPosition::new(
                        mouse_pos.x.clamp(window.left, window.right),
                        mouse_pos.y.clamp(window.top, window.bottom),
                    );

                    let selection =
                        Selection::from_points(self.selection_start, mouse_pos, self.window.scale);

                    self.capture.selection = selection;
                    self.selection_state = SelectionSate::Selecting;
                }
            }
            MouseState::Released => {
                match self.selection_state {
                    SelectionSate::Selecting => {
                        // Save selection
                        self.save().track()?;
                        self.close().track()?;
                        self.selection_state = SelectionSate::None;
                        return Ok(());
                    }
                    SelectionSate::StartedSelecting => {
                        //  Cancel selection
                        self.selection_state = SelectionSate::None;
                        return Ok(());
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }
}
