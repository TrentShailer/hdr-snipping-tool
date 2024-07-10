use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::RECT,
        Graphics::{Dxgi::DXGI_OUTPUT_DESC1, Gdi::HMONITOR},
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};
use winit::dpi::{PhysicalPosition, PhysicalSize};

use crate::enumerate_displays::DisplayConfig;

pub struct Display {
    pub handle: HMONITOR,
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
    pub sdr_whitepoint: f32,
    pub sdr_whitepoint_nits: f32,
    pub maximum_luminance: f32,
    pub capture_item: GraphicsCaptureItem,
}

impl Display {
    pub fn new(
        desc: &DXGI_OUTPUT_DESC1,
        config: &DisplayConfig,
    ) -> Result<Self, windows_result::Error> {
        let (position, size) = Self::position_size_from_rect(desc.DesktopCoordinates);
        let capture_item = Self::create_capture_item(desc.Monitor)?;

        Ok(Self {
            handle: desc.Monitor,
            position,
            size,
            maximum_luminance: desc.MaxLuminance,
            sdr_whitepoint: config.sdr_whitepoint,
            sdr_whitepoint_nits: config.sdr_whitepoint_nits,
            capture_item,
        })
    }

    pub fn update(&mut self, desc: &DXGI_OUTPUT_DESC1, config: &DisplayConfig) {
        let (position, size) = Self::position_size_from_rect(desc.DesktopCoordinates);

        self.position = position;
        self.size = size;
        self.maximum_luminance = desc.MaxLuminance;
        self.sdr_whitepoint = config.sdr_whitepoint;
        self.sdr_whitepoint_nits = config.sdr_whitepoint_nits;
    }

    pub fn position_size_from_rect(rect: RECT) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
        let width = (rect.right - rect.left).unsigned_abs();
        let height = (rect.bottom - rect.top).unsigned_abs();

        let size = PhysicalSize::new(width, height);
        let position = PhysicalPosition::new(rect.left, rect.top);

        (position, size)
    }

    pub fn create_capture_item(
        handle: HMONITOR,
    ) -> Result<GraphicsCaptureItem, windows_result::Error> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(handle)? };

        Ok(capture_item)
    }

    pub fn contains(&self, point: PhysicalPosition<i32>) -> bool {
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.width as i32
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.height as i32
    }
}

impl PartialEq<HMONITOR> for Display {
    fn eq(&self, other: &HMONITOR) -> bool {
        &self.handle == other
    }
}
