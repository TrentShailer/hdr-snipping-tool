use std::sync::mpsc::{channel, Receiver};

use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
};
use windows_core::IInspectable;

pub(crate) fn prepare_capture(
    capture_item: &GraphicsCaptureItem,
    dxgi_device: &IDirect3DDevice,
) -> windows_result::Result<(
    Direct3D11CaptureFramePool,
    GraphicsCaptureSession,
    Receiver<Direct3D11CaptureFrame>,
)> {
    let capture_size = capture_item.Size()?;

    // create frame pool
    let framepool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        dxgi_device,
        DirectXPixelFormat::R16G16B16A16Float,
        1,
        capture_size,
    )?;

    let session = framepool.CreateCaptureSession(capture_item)?;

    session.SetIsCursorCaptureEnabled(false)?;

    // setup sender and reciever for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    framepool.FrameArrived(
        &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
            move |frame_pool, _| {
                let frame_pool = match frame_pool {
                    Some(v) => v,
                    None => {
                        return Err(windows_core::Error::new(
                            windows_core::HRESULT(-1),
                            "Failed to access frame pool, frame_pool is None",
                        ))
                    }
                };

                let frame = frame_pool.TryGetNextFrame()?;
                sender
                    .send(frame)
                    .map_err(|e| windows_core::Error::new(windows_core::HRESULT(-1), e.to_string()))
            }
        }),
    )?;

    // Start capture
    session.StartCapture()?;

    Ok((framepool, session, receiver))
}
