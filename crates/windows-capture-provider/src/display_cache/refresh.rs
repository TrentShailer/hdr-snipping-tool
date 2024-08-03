use std::{collections::hash_map::Entry, time::Instant};

use crate::DirectXDevices;

use super::{get_displays::get_displays, DisplayCache, Error};

impl DisplayCache {
    /// Refreshes the display cache, purges disconnected displays,
    /// and adds new connected displays.
    pub fn refresh(&mut self, devices: &DirectXDevices) -> Result<(), Error> {
        let display_start = Instant::now();

        let current_displays = get_displays(devices)?;

        // Remove inactive displays from the capture item hashmap
        let old_displays: Box<[isize]> = self.capture_items.keys().cloned().collect();
        for old_handle in old_displays.iter() {
            if !current_displays
                .iter()
                .any(|display| display.handle.0 as isize == *old_handle)
            {
                self.capture_items.remove(old_handle);
            }
        }

        // Insert new displays into the hashmap
        for display in current_displays.iter() {
            if let Entry::Vacant(entry) = self.capture_items.entry(display.handle.0 as isize) {
                let capture_item = display
                    .create_capture_item()
                    .map_err(Error::CreateCaputreItem)?;

                entry.insert(capture_item);
            }
        }

        self.displays = current_displays;

        log::debug!(
            "[refresh_displays]
  [TIMING] {}ms",
            display_start.elapsed().as_millis()
        );

        Ok(())
    }
}
