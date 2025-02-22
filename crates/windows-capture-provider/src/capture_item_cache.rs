use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Graphics::Gdi::HMONITOR, System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};

use crate::{DirectX, LabelledWinResult, WinError};

/// A cache for monitors and their capture items.
pub struct CaptureItemCache {
    /// A map between HMONITOR handles and their capture item.
    capture_items: Vec<(HMONITOR, GraphicsCaptureItem)>,
}

impl CaptureItemCache {
    /// Creates a new capture item cache
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            capture_items: Vec::new(),
        }
    }

    /// Returns if the cache contains a given monitor.
    pub fn contains(&self, handle: HMONITOR) -> bool {
        self.capture_items
            .iter()
            .any(|(cache_handle, _)| *cache_handle == handle)
    }

    /// Gets the graphics capture item from the cache or creates and caches it if it doesn't exist.
    pub fn get_capture_item(&mut self, handle: HMONITOR) -> LabelledWinResult<GraphicsCaptureItem> {
        let maybe_capture_item = self
            .capture_items
            .iter()
            .find(|(cache_handle, _)| *cache_handle == handle);

        if let Some((_, capture_item)) = maybe_capture_item {
            return Ok(capture_item.clone());
        }

        let capture_item = Self::create_capture_item(handle)?;
        self.capture_items.push((handle, capture_item.clone()));

        Ok(capture_item)
    }

    /// Prunes the monitors in the cache that are no longer connected.
    pub fn prune(&mut self, direct_x: &DirectX) -> Result<(), WinError> {
        let output_descriptors = direct_x.dxgi_output_descriptors()?;

        self.capture_items.retain(|(cache_handle, _)| {
            output_descriptors
                .iter()
                .any(|descriptor| descriptor.Monitor == *cache_handle)
        });

        Ok(())
    }

    /// Caches any active monitors that aren't in the cache.
    pub fn cache_active(&mut self, direct_x: &DirectX) -> Result<(), WinError> {
        let output_descriptors = direct_x.dxgi_output_descriptors()?;

        // If a monitor does not exist in the cache, add it.
        for descriptor in output_descriptors {
            if self.contains(descriptor.Monitor) {
                continue;
            }

            self.capture_items.push((
                descriptor.Monitor,
                Self::create_capture_item(descriptor.Monitor)?,
            ));
        }

        Ok(())
    }

    /// Creates a graphics capture item for a monitor.
    pub fn create_capture_item(handle: HMONITOR) -> Result<GraphicsCaptureItem, WinError> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|e| WinError::new(e, "factory::GraphicsCaptureItem"))?;

        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(handle) }
            .map_err(|e| WinError::new(e, "GraphicsCaptureItem::CreateForMonitor"))?;

        Ok(capture_item)
    }
}
