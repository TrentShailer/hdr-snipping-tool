mod get_displays;
pub mod hovered;
pub mod refresh;

use std::collections::HashMap;

use get_displays::get_displays;
use thiserror::Error;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows_result::Error as WindowsError;

use crate::{DirectXDevices, Display};

/// A cache for the connected displays and their capture items.
pub struct DisplayCache {
    /// Set of currently active displays at the time of creation
    /// or last capture.
    pub displays: Box<[Display]>,

    /// A map between HMONITOR handles and their capture item.
    pub capture_items: HashMap<isize, GraphicsCaptureItem>,
}

impl DisplayCache {
    /// Create a new display cache and populate it.
    pub fn new(devices: &DirectXDevices) -> Result<Self, Error> {
        let displays = get_displays(devices)?;
        let mut capture_items = HashMap::new();

        for display in displays.iter() {
            let capture_item = display
                .create_capture_item()
                .map_err(Error::CreateCaputreItem)?;

            capture_items.insert(display.handle.0 as isize, capture_item);
        }

        Ok(Self {
            displays,
            capture_items,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get current displays:\n{0}")]
    GetDisplays(#[from] get_displays::Error),

    #[error("Failed to create capture item for display:\n{0}")]
    CreateCaputreItem(#[source] WindowsError),
}
