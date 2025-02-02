use windows::Graphics::Capture::GraphicsCaptureItem;

use crate::{send::SendHMONITOR, DirectX, LabelledWinResult, Monitor, MonitorError};

/// A cache for the connected displays and their capture items.
pub struct CaptureItemCache {
    /// A map between HMONITOR handles and their capture item.
    capture_items: Vec<(SendHMONITOR, GraphicsCaptureItem)>,
}

impl CaptureItemCache {
    /// Creates a new capture item cache
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            capture_items: vec![],
        }
    }

    /// Gets the graphics capture item from the cache or creates and caches it if it doesn't exist.
    pub fn get_capture_item(&mut self, monitor: Monitor) -> LabelledWinResult<GraphicsCaptureItem> {
        let maybe_capture_item = self
            .capture_items
            .iter()
            .find(|(handle, _)| handle.0 == monitor.handle.0);

        if let Some((_, capture_item)) = maybe_capture_item {
            return Ok(capture_item.clone());
        }

        let capture_item = monitor.create_capture_item()?;
        self.capture_items
            .push((monitor.handle, capture_item.clone()));

        Ok(capture_item)
    }

    /// Prunes the monitors in the cache that are no longer connected.
    pub fn prune(&mut self, direct_x: &DirectX) -> Result<(), MonitorError> {
        let monitors = Monitor::get_monitors(direct_x)?;

        self.capture_items
            .retain(|(handle, _)| monitors.iter().any(|monitor| monitor.handle.0 == handle.0));

        Ok(())
    }

    /// Caches any active monitors that aren't in the cache.
    pub fn cache_active(&mut self, direct_x: &DirectX) -> Result<(), MonitorError> {
        let monitors = Monitor::get_monitors(direct_x)?;

        // If a monitor does not exist in the cache, add it.
        for monitor in monitors {
            let exists_in_cache = self
                .capture_items
                .iter()
                .any(|(handle, _)| handle.0 == monitor.handle.0);

            if exists_in_cache {
                continue;
            }

            self.capture_items
                .push((monitor.handle, monitor.create_capture_item()?));
        }

        Ok(())
    }
}
