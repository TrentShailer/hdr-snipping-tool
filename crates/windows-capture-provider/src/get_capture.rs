use std::{sync::mpsc::RecvError, time::Instant};

use hdr_capture::{CaptureInfo, CaptureProvider, DisplayInfo};
use thiserror::Error;
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};
use winit::dpi::PhysicalPosition;

use crate::{
    enumerate_displays::refresh_displays, fetch_capture::fetch_capture, prepare_capture,
    WindowsCaptureProvider,
};

impl CaptureProvider for WindowsCaptureProvider {
    type Error = Error;

    fn get_capture(
        &mut self,
    ) -> Result<(Vec<u8>, hdr_capture::DisplayInfo, hdr_capture::CaptureInfo), Self::Error> {
        let start = Instant::now();

        self.displays = refresh_displays(&mut self.displays).map_err(Error::Displays)?;

        let mut mouse_point: POINT = Default::default();
        unsafe { GetCursorPos(&mut mouse_point) }.map_err(Error::Mouse)?;
        let mouse_pos = PhysicalPosition::new(mouse_point.x, mouse_point.y);

        let display = match self.displays.iter().find(|d| d.contains(mouse_pos)) {
            Some(v) => v,
            None => return Err(Error::NoDisplay),
        };

        let capture_item = match display.capture_item.as_ref() {
            Some(v) => v,
            None => return Err(Error::NoCaptureItem),
        };

        let (framepool, capture_session, capture_receiver) =
            prepare_capture(capture_item, &self.dxgi_device).map_err(Error::PrepareCapture)?;

        let frame = capture_receiver.recv()?;

        capture_session.Close().map_err(Error::CloseSession)?;
        framepool.Close().map_err(Error::CloseSession)?;

        let (raw_capture, capture_size) = fetch_capture(frame, &self.d3d_device, &self.d3d_context)
            .map_err(Error::FetchGapture)?;

        let capture_info = CaptureInfo::new(capture_size, display.position);
        let display_info = DisplayInfo::new(display.size, display.position);

        unsafe { self.d3d_context.ClearState() };
        self.dxgi_device.Trim().map_err(Error::Trim)?;

        let end = Instant::now();
        log::debug!("Got capture in {}ms", end.duration_since(start).as_millis());

        Ok((raw_capture, display_info, capture_info))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to enumerate displays:\n{0}")]
    Displays(#[source] windows_result::Error),

    #[error("Failed to get mouse position:\n{0}")]
    Mouse(#[source] windows_result::Error),

    #[error("Display had no capture item.")]
    NoCaptureItem,

    #[error("Couldn't find a display to capture.")]
    NoDisplay,

    #[error("Failed to prepare the frame capture:\n{0}")]
    PrepareCapture(#[source] windows_result::Error),

    #[error("Faile to fetch the capture:\n{0}")]
    FetchGapture(#[source] windows_result::Error),

    #[error("Failed to recieve the frame:\n{0}")]
    RecieveFrame(#[from] RecvError),

    #[error("Failed to close the capture session:\n{0}")]
    CloseSession(#[source] windows_result::Error),

    #[error("Failed to trim graphics memory:\n{0}")]
    Trim(#[source] windows_result::Error),
}
