use std::collections::hash_map::Entry;

use tracing::{info, instrument};
use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos},
};

use crate::Display;

use super::{CaptureItemCache, Error};

impl CaptureItemCache {
    /// Returns the display that the mouse is hovering over.
    #[instrument("CaptureItemCache::hovered", skip_all, err)]
    pub fn hovered(&mut self) -> Result<Option<(Display, GraphicsCaptureItem)>, Error> {
        let mut mouse_point: POINT = Default::default();
        unsafe { GetCursorPos(&mut mouse_point) }.map_err(Error::GetMouse)?;
        let mouse_pos = [mouse_point.x, mouse_point.y];

        // Retrieve display
        let maybe_display = self
            .displays
            .iter()
            .find(|display| display.contains(mouse_pos));
        let hovered_display = match maybe_display {
            Some(display) => *display,
            None => return Ok(None),
        };

        info!("{}", hovered_display);

        // Retrieve or create capture item
        let handle = (*hovered_display.handle).0 as isize;
        let capture_item = match self.capture_items.entry(handle) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(vacancy) => {
                info!("{}: creating capture item", handle);

                let capture_item = hovered_display
                    .create_capture_item()
                    .map_err(Error::CreateCaputreItem)?;
                vacancy.insert(capture_item).clone()
            }
        };

        Ok(Some((hovered_display, capture_item)))
    }
}
