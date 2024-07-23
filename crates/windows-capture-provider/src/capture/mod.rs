mod retrieve_capture;
mod select_display;
mod start_capture_session;

use std::{sync::mpsc::RecvError, time::Instant};

use select_display::select_display;
use start_capture_session::start_capture_session;
use thiserror::Error;
use windows_result::Error as WindowsError;

use crate::{display::Display, refresh_displays, WindowsCaptureProvider};
use retrieve_capture::retrieve_capture;

/// A capture and it's metadata.
pub struct Capture {
    /// The raw block of bytes that make up the capture, the data is in RGBA little endian f16.
    pub data: Box<[u8]>,

    /// The display the capture is of.
    pub display: Display,
}

impl WindowsCaptureProvider {
    /// Take a capture of the display the mouse is currently in.
    pub fn take_capture(&mut self) -> Result<Capture, Error> {
        let capture_start = Instant::now();

        self.refresh_displays()?;

        // find the display the mouse is in
        let display = select_display(&self.displays)?;
        log::debug!(
            "[select_display]
  {}",
            display
        );

        // get it's capture item
        let capture_item = self
            .display_capture_items
            .get(&display.handle.0)
            .ok_or(Error::NoCaptureItem)?;

        // get the framepool, capture session, and captuire receiver
        let (framepool, capture_session, capture_receiver) =
            start_capture_session(capture_item, &self.devices.d3d_device)?;

        // get the d3d_capture
        let recv_start = Instant::now();
        let d3d11_capture = capture_receiver.recv()?;
        log::debug!(
            "[recv_capture]
  [TIMING] {}ms",
            recv_start.elapsed().as_millis()
        );

        capture_session.Close().map_err(Error::CloseSession)?;
        framepool.Close().map_err(Error::CloseSession)?;

        // get the capture from gpu
        let capture = retrieve_capture(
            d3d11_capture,
            &self.devices.d3d11_device,
            &self.devices.d3d11_context,
        )
        .map_err(Error::FetchCapture)?;

        // free resources
        unsafe { self.devices.d3d11_context.ClearState() };
        self.devices.d3d_device.Trim().map_err(Error::Trim)?;

        log::debug!(
            "[take_capture]
  [TIMING] {}ms",
            capture_start.elapsed().as_millis()
        );

        Ok(Capture {
            data: capture,
            display: *display,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get refresh current displays:\n{0}")]
    RefreshDisplays(#[from] refresh_displays::Error),

    #[error("Failed to select a display to capture:\n{0}")]
    SelectDisplay(#[from] select_display::Error),

    #[error("Failed to get capture item for display")]
    NoCaptureItem,

    #[error("Failed to start capture session:\n{0}")]
    StartCaptureSession(#[from] start_capture_session::Error),

    #[error("Failed to receive capture\n{0}")]
    RecvFrame(#[from] RecvError),

    #[error("Failed to close the capture session\n{0}")]
    CloseSession(#[source] WindowsError),

    #[error("Failed to fetch the capture from the GPU\n{0}")]
    FetchCapture(#[source] WindowsError),

    #[error("Failed to trim d3d memory\n{0}")]
    Trim(#[source] WindowsError),
}
