use thiserror::Error;
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
use windows_result::Error as WindowsError;

use crate::Display;

use super::DisplayCache;

impl DisplayCache {
    /// Returns the display that the mouse is hovering over.
    pub fn hovered(&self) -> Result<Option<Display>, Error> {
        let mut mouse_point: POINT = Default::default();
        unsafe { GetCursorPos(&mut mouse_point) }.map_err(Error::GetMouse)?;
        let mouse_pos = [mouse_point.x, mouse_point.y];

        let display = self
            .displays
            .iter()
            .find(|display| display.contains(mouse_pos));

        Ok(display.copied())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get mouse pos:\n{0}")]
    GetMouse(#[source] WindowsError),
}
