use tracing::{info, instrument};

use crate::DirectXDevices;

use super::{get_displays::get_displays, CaptureItemCache, Error};

impl CaptureItemCache {
    /// Refreshes the capture item cache, purges disconnected displays.
    /// New capture items are only created on request.
    #[instrument("CaptureItemCache::refresh_displays", skip_all, err)]
    pub fn refresh_displays(&mut self, devices: &DirectXDevices) -> Result<(), Error> {
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

        self.displays = current_displays;

        Ok(())
    }
}
