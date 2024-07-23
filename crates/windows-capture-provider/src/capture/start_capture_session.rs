use std::{
    sync::mpsc::{channel, Receiver},
    time::Instant,
};

use thiserror::Error;
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::UI::WindowsAndMessaging::WM_APP,
};
use windows_core::{IInspectable, HRESULT};
use windows_result::Error as WindowsError;

pub fn start_capture_session(
    capture_item: &GraphicsCaptureItem,
    d3d_device: &IDirect3DDevice,
) -> Result<
    (
        Direct3D11CaptureFramePool,
        GraphicsCaptureSession,
        Receiver<Direct3D11CaptureFrame>,
    ),
    Error,
> {
    let start = Instant::now();

    let capture_size = capture_item.Size().map_err(Error::CaptureSize)?;

    let framepool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        d3d_device,
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

    log::debug!(
        "[start_capture_session]
  [TIMING] {}ms",
        start.elapsed().as_millis()
    );

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
