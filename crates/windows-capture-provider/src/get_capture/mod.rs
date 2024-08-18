mod retrieve_capture;
mod start_capture_session;

use std::sync::mpsc::RecvError;

use thiserror::Error;
use tracing::info_span;
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows_result::Error as WindowsError;

use retrieve_capture::retrieve_capture;
use start_capture_session::start_capture_session;

use crate::{capture::Capture, DirectXDevices, Display};

/// Get a capture of a given display.
pub fn get_capture(
    devices: &DirectXDevices,
    display: &Display,
    capture_item: &GraphicsCaptureItem,
) -> Result<Capture, Error> {
    let _span = info_span!("get_capture").entered();

    // get the framepool, capture session, and captuire receiver
    let (framepool, capture_session, capture_receiver) =
        start_capture_session(devices, capture_item)?;

    // get the d3d_capture
    let recv_span = info_span!("recv").entered();
    let d3d11_capture = capture_receiver.recv()?;
    recv_span.exit();

    capture_session.Close().map_err(Error::CloseSession)?;
    framepool.Close().map_err(Error::CloseSession)?;

    // get the capture from gpu
    let capture_handle = retrieve_capture(d3d11_capture).map_err(Error::FetchCapture)?;

    // free resources
    unsafe { devices.d3d11_context.ClearState() };
    devices.d3d_device.Trim().map_err(Error::Trim)?;

    Ok(Capture {
        handle: capture_handle,
        display: *display,
    })
}

#[derive(Debug, Error)]
pub enum Error {
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
