mod get_monitors;
mod sdr_white;

use windows::Win32::{Foundation::RECT, Graphics::Dxgi::DXGI_OUTPUT_DESC1};
use windows_core::PCWSTR;

use crate::{WinError, send::SendHMONITOR};

/// A monitor and related data
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Monitor {
    /// The monitor's `HMONITOR` handle.
    pub handle: SendHMONITOR,

    /// The monitor's desktop coordinates rect. Relative to the top-left point of the primary
    /// monitor.
    pub desktop_coordinates: RECT,

    /// The monitor's GDI device name.
    pub device_name: [u16; 32],

    /// The monitor's maximum luminance.
    pub min_brightness: f32,

    /// The monitor's minimum luminance.
    pub max_brightness: f32,

    /// The monitor's SDR White luminance.
    pub sdr_white: f32,
}

impl Monitor {
    /// Create a new monitor, returns `Ok(None)` if the monitor is inactive.
    pub(crate) fn new(descriptor: DXGI_OUTPUT_DESC1) -> Result<Option<Self>, WinError> {
        let sdr_white = match Self::get_sdr_white(descriptor)? {
            Some(sdr_white) => sdr_white,
            None => return Ok(None),
        };

        Ok(Some(Self {
            handle: SendHMONITOR(descriptor.Monitor),
            desktop_coordinates: descriptor.DesktopCoordinates,
            device_name: descriptor.DeviceName,
            max_brightness: descriptor.MaxLuminance / 80.0,
            min_brightness: descriptor.MinLuminance / 80.0,
            sdr_white,
        }))
    }

    /// Calculates the monitor's width and height from it's Desktop Coordinates.
    pub fn size(&self) -> [u32; 2] {
        let rect = self.desktop_coordinates;

        let width = rect.left.abs_diff(rect.right);
        let height = rect.top.abs_diff(rect.bottom);

        [width, height]
    }
}

impl Eq for Monitor {}
impl PartialEq for Monitor {
    fn eq(&self, other: &Self) -> bool {
        self.handle.0 == other.handle.0
    }
}

impl core::fmt::Debug for Monitor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = unsafe { PCWSTR::from_raw(self.device_name.as_ptr()).to_string() }
            .unwrap_or("Invalid Name".to_string());

        f.debug_struct("Monitor")
            .field("handle", &self.handle.0)
            .field("desktop_coordinates", &self.desktop_coordinates)
            .field("device_name", &name)
            .field("min_brightness", &self.min_brightness)
            .field("max_brightness", &self.max_brightness)
            .field("sdr_white", &self.sdr_white)
            .finish()
    }
}
