mod sdr_white;

use core::fmt::Debug;

use thiserror::Error;
use tracing::debug;
use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::{POINT, RECT},
        Graphics::Dxgi::{IDXGIOutput6, DXGI_OUTPUT_DESC1},
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        UI::WindowsAndMessaging::GetCursorPos,
    },
};
use windows_core::Interface;

use crate::{send::SendHMONITOR, DirectX, LabelledWinResult, WinError};

/// A monitor and related data
#[derive(Debug, Clone, Copy)]
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
    /// Create a new monitor.
    pub(crate) fn new(descriptor: DXGI_OUTPUT_DESC1) -> Result<Self, Error> {
        let sdr_white = Self::get_sdr_white(descriptor)?;

        Ok(Self {
            handle: SendHMONITOR(descriptor.Monitor),
            desktop_coordinates: descriptor.DesktopCoordinates,
            device_name: descriptor.DeviceName,
            max_brightness: descriptor.MaxLuminance / 80.0,
            min_brightness: descriptor.MinLuminance / 80.0,
            sdr_white,
        })
    }

    /// Returns the DXGI device's current outputs.
    pub fn get_monitors(direct_x: &DirectX) -> Result<Vec<Self>, Error> {
        let mut monitors = vec![];

        let mut output_index = 0;
        while let Ok(output) = unsafe { direct_x.dxgi_adapter.EnumOutputs(output_index) } {
            let output_6: IDXGIOutput6 = output
                .cast()
                .map_err(|e| WinError::new(e, "IDXGIOutput::cast"))?;

            let output_desc_1 = unsafe { output_6.GetDesc1() }
                .map_err(|e| WinError::new(e, "IDXGIOutput6::GetDesc1"))?;

            monitors.push(Self::new(output_desc_1)?);

            output_index += 1;
        }

        Ok(monitors)
    }

    /// Returns the monitor that is currently hovered by the mouse.
    pub fn get_hovered_monitor(direct_x: &DirectX) -> Result<Option<Self>, Error> {
        let monitors = Self::get_monitors(direct_x)?;

        let mut mouse_point: POINT = Default::default();
        unsafe { GetCursorPos(&mut mouse_point) }.map_err(|e| WinError::new(e, "GetCursorPos"))?;

        let monitor = monitors.into_iter().find(|monitor| {
            let left = monitor.desktop_coordinates.left;
            let right = monitor.desktop_coordinates.right;
            let top = monitor.desktop_coordinates.top;
            let bottom = monitor.desktop_coordinates.bottom;

            mouse_point.x >= left
                && mouse_point.x <= right
                && mouse_point.y >= top
                && mouse_point.y <= bottom
        });

        Ok(monitor)
    }

    /// Creates a graphics capture item for this monitor.
    pub fn create_capture_item(&self) -> LabelledWinResult<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|e| WinError::new(e, "factory::GraphicsCaptureItem"))?;

        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(self.handle.0) }
            .map_err(|e| WinError::new(e, "GraphicsCaptureItem::CreateForMonitor"))?;

        Ok(capture_item)
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

impl core::fmt::Display for Monitor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Monitor {{ handle: {:?}, rect: {:?}, sdr_white: {}, min_brightness: {}, max_brightness: {} }}",
            self.handle.0, self.desktop_coordinates, self.sdr_white, self.min_brightness, self.max_brightness
        )
    }
}

#[derive(Debug, Error)]
/// Error variants from creating a monitor.
pub enum Error {
    #[error(transparent)]
    /// A Windows API call failed.
    WinError(#[from] WinError),

    #[error("The DXGI Outputs and monitor config do not match.")]
    /// The monitor had no matching config path.
    MonitorsMismatch,
}
