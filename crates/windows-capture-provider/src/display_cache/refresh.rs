use std::collections::hash_map::Entry;

use tracing::{info, info_span};

use crate::DirectXDevices;

use super::{get_displays::get_displays, DisplayCache, Error};

impl DisplayCache {
    /// Refreshes the display cache, purges disconnected displays,
    /// and adds new connected displays.
    pub fn refresh(&mut self, devices: &DirectXDevices) -> Result<(), Error> {
        let _span = info_span!("DisplayCache::refresh").entered();

        let current_displays = get_displays(devices)?;

        // Remove inactive displays from the capture item hashmap
        let old_displays: Box<[isize]> = self.capture_items.keys().cloned().collect();
        for old_handle in old_displays.iter() {
            if !current_displays
                .iter()
                .any(|display| display.handle.0 as isize == *old_handle)
            {
                info!("{}: removed capture item", old_handle);
                self.capture_items.remove(old_handle);
            }
        }

        // Insert new displays into the hashmap
        for display in current_displays.iter() {
            let handle = display.handle.0 as isize;
            if let Entry::Vacant(entry) = self.capture_items.entry(handle) {
                let capture_item = display
                    .create_capture_item()
                    .map_err(Error::CreateCaputreItem)?;

                info!("{}: created capture item", handle);
                entry.insert(capture_item);
            }
        }

        self.displays = current_displays;

        Ok(())
    }
}
