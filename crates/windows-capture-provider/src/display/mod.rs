pub mod get_current_displays;

use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::RECT,
        Graphics::{
            Direct2D::D2D1_SCENE_REFERRED_SDR_WHITE_LEVEL, Dxgi::DXGI_OUTPUT_DESC1, Gdi::HMONITOR,
        },
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};
use windows_result::Result as WindowsResult;

/// A windows display and related data
#[derive(Debug, Clone, Copy)]
pub struct Display {
    /// The display handle.
    pub handle: HMONITOR,

    /// The position of the top left corner of the display in pixels.
    /// This is relative to the top left corner of the primary display.
    pub position: [i32; 2],

    /// The size of the display in pixels.
    pub size: [u32; 2],

    /// The maximum luminance of the display in nits.
    pub luminance: f32,

    /// The display's SDR reference white in nits.
    pub sdr_referece_white: f32,
}

impl Display {
    /// Create a display object from a `DXGI_OUTPUT_DESC1` and sdr reference white.
    pub fn from_desc1(desc: &DXGI_OUTPUT_DESC1, sdr_referece_white: f32) -> Self {
        let (position, size) = Self::position_size_from_rect(desc.DesktopCoordinates);

        Self {
            handle: desc.Monitor,
            position,
            size,
            luminance: desc.MaxLuminance,
            sdr_referece_white,
        }
    }

    /// Calculate what the encoded sdr reference white is.
    pub fn encoded_sdr_reference_white(&self) -> f32 {
        self.sdr_referece_white / D2D1_SCENE_REFERRED_SDR_WHITE_LEVEL
    }

    /// Returns whether a point is contained within the bounds of the display.
    pub fn contains(&self, point: [i32; 2]) -> bool {
        let left = self.position[0];
        let right = self.position[0] + self.size[0] as i32;
        let top = self.position[1];
        let bottom = self.position[1] + self.size[1] as i32;

        point[0] >= left && point[0] <= right && point[1] >= top && point[1] <= bottom
    }

    /// Creates a graphics capture item for this display.
    pub fn create_capture_item(&self) -> WindowsResult<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(self.handle)? };

        Ok(capture_item)
    }

    /// Returns the position and size of a rect.
    fn position_size_from_rect(rect: RECT) -> ([i32; 2], [u32; 2]) {
        let position = [rect.left, rect.top];

        let width = (rect.right - rect.left).unsigned_abs();
        let height = (rect.bottom - rect.top).unsigned_abs();
        let size = [width, height];

        (position, size)
    }
}

impl PartialEq for Display {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Eq for Display {}
