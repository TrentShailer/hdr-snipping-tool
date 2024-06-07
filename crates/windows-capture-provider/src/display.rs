use windows::{
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Graphics::Gdi::HMONITOR, System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
    },
};
use winit::dpi::{PhysicalPosition, PhysicalSize};

#[derive(Eq)]
pub struct Display {
    pub handle: HMONITOR,
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
    pub capture_item: Option<GraphicsCaptureItem>,
}

impl Display {
    pub fn new(handle: HMONITOR, position: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Self {
        Self {
            handle,
            position,
            size,
            capture_item: None,
        }
    }

    pub fn create_capture_item(&mut self) -> Result<(), windows_result::Error> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(self.handle)? };
        self.capture_item = Some(capture_item);

        Ok(())
    }

    pub fn contains(&self, point: PhysicalPosition<i32>) -> bool {
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.width as i32
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.height as i32
    }
}

impl PartialEq for Display {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.position == other.position && self.size == other.size
    }
}
