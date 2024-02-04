use std::sync::mpsc::channel;

use anyhow::Context;
use display::DisplayInfo;
use windows::core::{ComInterface, IInspectable};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use super::texture::get_texture_from_surface;
use crate::image::Image;

use super::d3d_device::{create_d3d_device, create_dxgi_device};
use super::display::{self, get_display};

pub fn get_capture() -> anyhow::Result<(Image, DisplayInfo)> {
    // create d3d device for capture item
    let d3d_device = create_d3d_device().context("Failed to create d3d_device")?;
    let d3d_context = unsafe {
        d3d_device
            .GetImmediateContext()
            .context("Failed to get d3d_device context.")?
    };
    let dxgi_device = create_dxgi_device(&d3d_device).context("Failed to create dxgi_device")?;

    let display = get_display().context("Failed to get display")?;

    // turn display into capture item
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .context("Failed to create graphics capture interop")?;
    let capture_item: GraphicsCaptureItem = unsafe {
        interop
            .CreateForMonitor(display.handle)
            .context("Failed to create interop for display")?
    };

    let capture_size = capture_item.Size().context("Failed to get capture size")?;

    // create frame pool
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &dxgi_device,
        DirectXPixelFormat::R16G16B16A16Float,
        1,
        capture_size,
    )
    .context("Failed to create frame pool")?;
    let session = frame_pool
        .CreateCaptureSession(&capture_item)
        .context("Failed to create capture session")?;

    session
        .SetIsCursorCaptureEnabled(false)
        .context("Failed to remove cursor from capture")?;

    // setup sender and reciever for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    frame_pool
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
        .context("Failed to setup frame arrival")?;

    // Start capture
    session.StartCapture().context("Failed to start capture.")?;

    // wait for frame
    let frame = receiver.recv().context("Failed to recieve frame")?;

    // Copy frame into new texture
    let texture = unsafe {
        let source_texture: ID3D11Texture2D =
            get_texture_from_surface(&frame.Surface().context("Failed to get frame surface")?)
                .context("failed to get texture from surface")?;

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut desc);
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

        let copy_texture = {
            let mut texture = None;
            d3d_device
                .CreateTexture2D(&desc, None, Some(&mut texture))
                .context("Failed to create texture2D")?;
            texture.unwrap()
        };

        d3d_context.CopyResource(
            Some(&copy_texture.cast().context("Failed to cast copy_texture")?),
            Some(
                &source_texture
                    .cast()
                    .context("Failed to cast source_texture")?,
            ),
        );

        session.Close().context("Failed to close capture session")?;
        frame_pool.Close().context("Failed to close frame pool")?;

        copy_texture
    };

    let image = unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture
            .cast()
            .context("Failed to cast texture to resource")?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        d3d_context
            .Map(
                Some(&resource.clone()),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            )
            .context("Failed to map texture resource")?;

        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        // TODO find out why capture is returning more data then neccecary
        let image: Image = Image::from_u8(
            slice,
            slice.len() as u32 / desc.Height / 4 / 2, /*  desc.Width as usize */
            desc.Height,
        );

        d3d_context.Unmap(Some(&resource), 0);

        image
    };

    Ok((image, display))
}
