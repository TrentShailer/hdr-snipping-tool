use std::sync::mpsc::{channel, Receiver};

use snafu::{ResultExt, Snafu};
use windows::{
    core::IInspectable,
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
};

use super::get_display::WindowsDisplay;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Windows API call '{call}' returned an error."))]
    WindowsApi {
        source: windows::core::Error,
        call: &'static str,
    },
}

pub fn prepare_capture(
    display: &WindowsDisplay,
    dxgi_device: &IDirect3DDevice,
) -> Result<
    (
        Direct3D11CaptureFramePool,
        GraphicsCaptureSession,
        Receiver<Direct3D11CaptureFrame>,
    ),
    Error,
> {
    // turn display into capture item
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .context(WindowsApiSnafu {
            call: "windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()",
        })?;

    let capture_item: GraphicsCaptureItem = unsafe {
        interop
            .CreateForMonitor(display.handle)
            .context(WindowsApiSnafu {
                call: "interop.CreateForMonitor(display.handle)",
            })?
    };

    let capture_size = capture_item.Size().context(WindowsApiSnafu {
        call: "capture_item.Size()",
    })?;

    // create frame pool
    let framepool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        dxgi_device,
        DirectXPixelFormat::R16G16B16A16Float,
        1,
        capture_size,
    )
    .context(WindowsApiSnafu {
        call: "Direct3D11CaptureFramePool::CreateFreeThreaded",
    })?;

    let session = framepool
        .CreateCaptureSession(&capture_item)
        .context(WindowsApiSnafu {
            call: "CreateCaptureSession",
        })?;

    session
        .SetIsCursorCaptureEnabled(false)
        .context(WindowsApiSnafu {
            call: "SetIsCursorCaptureEnabled",
        })?;

    // setup sender and reciever for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    framepool
        .FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame()?;
                    sender.send(frame).expect("Failed to send frame.");
                    Ok(())
                }
            }),
        )
        .context(WindowsApiSnafu {
            call: "framepool.FrameArrived",
        })?;

    // Start capture
    session.StartCapture().context(WindowsApiSnafu {
        call: "StartCapture",
    })?;

    Ok((framepool, session, receiver))
}
