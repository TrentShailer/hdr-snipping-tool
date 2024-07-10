pub mod capture;
pub mod directx_devices;
pub mod display;
pub mod refresh_displays;

use std::collections::HashMap;

use directx_devices::DirectXDevices;
use display::{
    get_current_displays::{self, get_current_displays},
    Display,
};

use thiserror::Error;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows_result::Error as WindowsError;

/// Structure to obtain captures on windows.
pub struct WindowsCaptureProvider {
    /// DirectX devices used through the capture process.
    pub devices: DirectXDevices,

    /// Set of currently active displays at the time of creation
    /// or last capture.
    pub displays: Box<[Display]>,

    /// A mapp between HMONITOR handles and their capture item,
    /// this is updated whenever displays are updated.
    pub display_capture_items: HashMap<isize, GraphicsCaptureItem>,
}

impl WindowsCaptureProvider {
    /// Create a new windows capture provider.
    pub fn new() -> Result<Self, Error> {
        let devices = DirectXDevices::new()?;
        let displays = get_current_displays(&devices.dxgi_adapter)?;
        let mut display_capture_items = HashMap::new();

        for display in displays.iter() {
            let capture_item = display
                .create_capture_item()
                .map_err(Error::CreateCaputreItem)?;

            display_capture_items.insert(display.handle.0, capture_item);
        }

        Ok(Self {
            devices,
            displays,
            display_capture_items,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create directX devices:\n{0}")]
    CreateDevices(#[from] directx_devices::Error),

    #[error("Failed to get current displays:\n{0}")]
    GetDisplays(#[from] get_current_displays::Error),

    #[error("Failed to create capture item for display:\n{0}")]
    CreateCaputreItem(#[source] WindowsError),
}
