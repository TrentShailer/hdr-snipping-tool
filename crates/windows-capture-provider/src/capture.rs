use std::sync::mpsc::channel;

use tracing::error;
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem},
        DirectX::DirectXPixelFormat,
    },
    Win32::{
        Graphics::{
            Direct3D11::ID3D11Texture2D,
            Dxgi::{IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE},
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};
use windows_core::{IInspectable, Interface};

use crate::{DirectX, LabelledWinResult, SendHANDLE, WinError};

/// A capture from Windows in R16G16B16A16_Float format
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct WindowsCapture {
    /// Handle to the shared Dx11 texture resource.
    pub handle: SendHANDLE,

    /// The size of the capture.
    pub size: [u32; 2],
}

/// The resources leftover from taking a Windows capture.
pub struct WindowsCaptureResources {
    frame: Direct3D11CaptureFrame,
    framepool: Direct3D11CaptureFramePool,
}

impl WindowsCapture {
    /// Take and retrieve a WindowsCapture from a capture item.
    pub fn take_capture(
        direct_x: &DirectX,
        capture_item: &GraphicsCaptureItem,
    ) -> LabelledWinResult<(Self, WindowsCaptureResources)> {
        // Get the capture size
        let capture_size = capture_item
            .Size()
            .map_err(|e| WinError::new(e, "GraphicsCaptureItem::Size"))?;

        // Setup sender and receiver the frame arrived event.
        let (frame_arrived_sender, frame_arrived) = channel();

        // Create the framepool.
        let framepool = {
            let framepool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &direct_x.d3d_device,
                DirectXPixelFormat::R16G16B16A16Float,
                1,
                capture_size,
            )
            .map_err(|e| WinError::new(e, "Direct3D11CaptureFramePool::CreateFreeThreaded"))?;

            // Handle frame arrived event
            framepool
                .FrameArrived(
                    &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                        move |_, _| {
                            match frame_arrived_sender.send(()) {
                                Ok(_) => {}
                                Err(e) => error!("Failed to send frame arrived event: {e}"),
                            };

                            Ok(())
                        }
                    }),
                )
                .map_err(|e| WinError::new(e, "Direct3D11CaptureFramePool::FrameArrived"))?;

            framepool
        };

        // Create and start the capture session
        let session = {
            let session = framepool.CreateCaptureSession(capture_item).map_err(|e| {
                WinError::new(e, "Direct3D11CaptureFramePool::CreateCaptureSession")
            })?;

            session.SetIsCursorCaptureEnabled(false).map_err(|e| {
                WinError::new(e, "GraphicsCaptureSession::SetIsCursorCaptureEnabled")
            })?;

            session
                .StartCapture()
                .map_err(|e| WinError::new(e, "GraphicsCaptureSession::StartCapture"))?;

            session
        };

        // Get the frame
        let frame = {
            // Wait for the frame arrived event to trigger on the framepool.
            frame_arrived
                .recv()
                .expect("Framepool dropped the frame arrived sender before sending");

            framepool
                .TryGetNextFrame()
                .map_err(|e| WinError::new(e, "Direct3D11CaptureFramePool::TryGetNextFrame"))?
        };

        // Retreive the frame's handle
        let handle = {
            // Get texture resource of the frame
            let texture = {
                let surface = frame
                    .Surface()
                    .map_err(|e| WinError::new(e, "Direct3D11CaptureFrame::Surface"))?;

                let access: IDirect3DDxgiInterfaceAccess = surface
                    .cast()
                    .map_err(|e| WinError::new(e, "IDirect3DSurface::cast"))?;

                unsafe { access.GetInterface::<ID3D11Texture2D>() }
                    .map_err(|e| WinError::new(e, "IDirect3DDxgiInterfaceAccess::GetInterface"))?
            };

            // Create and return handle to texture
            unsafe {
                let shared_resource: IDXGIResource1 = texture
                    .cast()
                    .map_err(|e| WinError::new(e, "ID3D11Texture2D::cast"))?;

                shared_resource
                    .CreateSharedHandle(
                        None,
                        (DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE).0,
                        None,
                    )
                    .map_err(|e| WinError::new(e, "IDXGIResource1::CreateSharedHandle"))?
            }
        };

        // Clean up
        {
            session
                .Close()
                .map_err(|e| WinError::new(e, "GraphicsCaptureSession::Close"))?;
        }

        Ok((
            Self {
                handle: SendHANDLE(handle),
                size: [
                    capture_size.Width.unsigned_abs(),
                    capture_size.Height.unsigned_abs(),
                ],
            },
            WindowsCaptureResources { frame, framepool },
        ))
    }
}

impl WindowsCaptureResources {
    /// Destroy the resources created to retreive the capture.
    pub fn destroy(&self, direct_x: &DirectX) -> LabelledWinResult<()> {
        self.frame
            .Close()
            .map_err(|e| WinError::new(e, "Direct3D11CaptureFrame::Close"))?;

        self.framepool
            .Close()
            .map_err(|e| WinError::new(e, "Direct3D11CaptureFramePool::Close"))?;

        unsafe { direct_x.d3d11_context.ClearState() }
        direct_x
            .d3d_device
            .Trim()
            .map_err(|e| WinError::new(e, "IDirect3DDevice::Trim"))?;

        Ok(())
    }
}
