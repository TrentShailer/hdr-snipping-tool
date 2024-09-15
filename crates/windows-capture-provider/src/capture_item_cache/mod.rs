mod get_displays;
pub mod hovered;
pub mod refresh;

use std::collections::HashMap;

use get_displays::get_displays;
use thiserror::Error;
use tracing::instrument;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows_result::Error as WindowsError;

use crate::{DirectXDevices, Display};

/// A cache for the connected displays and their capture items.
pub struct CaptureItemCache {
    /// Set of currently active displays at the time of creation
    /// or last capture.
    displays: Box<[Display]>,

    /// A map between HMONITOR handles and their capture item.
    capture_items: HashMap<isize, GraphicsCaptureItem>,
}

impl CaptureItemCache {
    /// Create a new display cache and populate it.
    #[instrument("CaptureItemCache::new", skip_all, err)]
    pub fn new(devices: &DirectXDevices) -> Result<Self, Error> {
        let displays = get_displays(devices)?;
        let mut capture_items = HashMap::new();

        for display in displays.iter() {
            let capture_item = display
                .create_capture_item()
                .map_err(Error::CreateCaputreItem)?;

            capture_items.insert((*display.handle).0 as isize, capture_item);
        }

        Ok(Self {
            displays,
            capture_items,
        })
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to get current displays:\n{0}")]
    GetDisplays(#[from] get_displays::Error),

    #[error("Failed to create capture item for display:\n{0}")]
    CreateCaputreItem(#[source] WindowsError),

    #[error("Failed to get mouse position:\n{0}")]
    GetMouse(#[source] WindowsError),
}
