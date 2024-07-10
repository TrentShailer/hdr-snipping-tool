use thiserror::Error;
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
use windows_result::Error as WindowsError;

use crate::display::Display;

pub fn select_display(displays: &[Display]) -> Result<&Display, Error> {
    let mut mouse_point: POINT = Default::default();
    unsafe { GetCursorPos(&mut mouse_point) }.map_err(Error::GetMouse)?;
    let mouse_pos = [mouse_point.x, mouse_point.y];

    let maybe_display = displays.iter().find(|d| d.contains(mouse_pos));

    maybe_display.ok_or(Error::NoDisplay)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get mouse position:\n{0}")]
    GetMouse(#[source] WindowsError),

    #[error("Failed to find a display containing the mouse")]
    NoDisplay,
}
