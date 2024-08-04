use std::sync::mpsc::{channel, Receiver};

use thiserror::Error;
use tracing::info_span;
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::DirectXPixelFormat,
    },
    Win32::UI::WindowsAndMessaging::WM_APP,
};
use windows_core::{IInspectable, HRESULT};
use windows_result::Error as WindowsError;

use crate::DirectXDevices;

/// Start a capture session for a given capture item.\
/// Returns the frame pool, the capture session, and a receiver for the capture frame.
pub fn start_capture_session(
    devices: &DirectXDevices,
    capture_item: &GraphicsCaptureItem,
) -> Result<
    (
        Direct3D11CaptureFramePool,
        GraphicsCaptureSession,
        Receiver<Direct3D11CaptureFrame>,
    ),
    Error,
> {
    let _span = info_span!("start_capture_session").entered();

    let capture_size = capture_item.Size().map_err(Error::CaptureSize)?;

    let framepool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &devices.d3d_device,
        DirectXPixelFormat::R16G16B16A16Float,
        1,
        capture_size,
    )
    .map_err(Error::Framepool)?;

    let session = framepool
        .CreateCaptureSession(capture_item)
        .map_err(Error::CreateCaptureSession)?;

    session
        .SetIsCursorCaptureEnabled(false)
        .map_err(Error::CursorCapture)?;

    // setup sender and receiver for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    framepool
        .FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().ok_or(WindowsError::new(
                        HRESULT::from_win32(WM_APP),
                        "Failed to access frame pool, frame_pool is None",
                    ))?;

                    let frame = frame_pool.TryGetNextFrame()?;
                    sender
                        .send(frame)
                        .map_err(|e| WindowsError::new(HRESULT::from_win32(WM_APP), e.to_string()))
                }
            }),
        )
        .map_err(Error::FrameArrived)?;

    session.StartCapture().map_err(Error::StartCapture)?;

    Ok((framepool, session, receiver))
}
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get capture item size:\n{0}")]
    CaptureSize(#[source] WindowsError),

    #[error("Failed to create framepool:\n{0}")]
    Framepool(#[source] WindowsError),

    #[error("Failed to create capture session:\n{0}")]
    CreateCaptureSession(#[source] WindowsError),

    #[error("Failed to set cursor capture:\n{0}")]
    CursorCapture(#[source] WindowsError),

    #[error("Failed to handle frame arrival:\n{0}")]
    FrameArrived(#[source] WindowsError),

    #[error("Failed to start capture session:\n{0}")]
    StartCapture(#[source] WindowsError),
}
