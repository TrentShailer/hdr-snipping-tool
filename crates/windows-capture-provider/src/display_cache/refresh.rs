use std::{collections::hash_map::Entry, time::Instant};

use crate::DirectXDevices;

use super::{get_displays::get_displays, DisplayCache, Error};

impl DisplayCache {
    /// Refreshes the display cache, purges disconnected displays,
    /// and adds new connected displays.
    pub fn refresh(&mut self, devices: &DirectXDevices) -> Result<(), Error> {
        let display_start = Instant::now();

        let displays = get_displays(devices)?;

        // Remove inactive displays from the capture item hashmap
        let keys: Box<[isize]> = self.capture_items.keys().cloned().collect();
        for handle in keys.iter() {
            if !displays.iter().any(|d| d.handle.0 == *handle) {
                self.capture_items.remove(handle);
            }
        }

        // Insert new displays into the hashmap
        for display in displays.iter() {
            if let Entry::Vacant(entry) = self.capture_items.entry(display.handle.0) {
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
