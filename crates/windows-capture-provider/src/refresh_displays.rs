use std::{collections::hash_map::Entry, time::Instant};

use thiserror::Error;
use windows_result::Error as WindowsError;

use crate::{
    display::get_current_displays::{self, get_current_displays},
    WindowsCaptureProvider,
};

impl WindowsCaptureProvider {
    /// Refreshes the stored displays and capture items to match the currently active displays.
    pub fn refresh_displays(&mut self) -> Result<(), Error> {
        let display_start = Instant::now();

        let displays = get_current_displays(&self.devices.dxgi_adapter)?;

        // Remove inactive displays from the capture item hashmap
        let keys: Box<[isize]> = self.display_capture_items.keys().cloned().collect();
        for handle in keys.iter() {
            if displays.iter().any(|d| d.handle.0 == *handle) {
                self.display_capture_items.remove(handle);
            }
        }

        // Insert new displays into the hashmap
        for display in displays.iter() {
            if let Entry::Vacant(entry) = self.display_capture_items.entry(display.handle.0) {
                let capture_item = display
                    .create_capture_item()
                    .map_err(Error::CreateCaputreItem)?;

                entry.insert(capture_item);
            }
        }

        self.displays = displays;

        log::debug!(
            "[refresh_displays]
  [TIMING] {}ms",
            display_start.elapsed().as_millis()
        );

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get current displays:\n{0}")]
    GetDisplays(#[from] get_current_displays::Error),

    #[error("Failed to create capture item for display:\n{0}")]
    CreateCaputreItem(#[source] WindowsError),
}
