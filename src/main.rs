mod d3d_device;
mod display;
mod logger;
mod texture;

use std::sync::mpsc::channel;

use log::{error, info};
use windows::core::{ComInterface, IInspectable, Result, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat};
use windows::Storage::{CreationCollisionOption, FileAccessMode, StorageFolder};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use crate::d3d_device::{create_d3d_device, create_dxgi_device};
use crate::display::get_display;
use crate::texture::get_texture_from_surface;

fn main() -> Result<()> {
    logger::init_fern().unwrap();

    if !GraphicsCaptureSession::IsSupported()? {
        error!("Graphics capture is not supported.");
        return Ok(());
    }
    info!("Graphics capture is supported.");

    // Loop, get keybind down
    // from mouse get the display
    let display = get_display()?;

    // turn display into capture item
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    let capture_item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(display.handle) }?;
    let capture_size = capture_item.Size()?;

    // create d3d device for capture item
    let d3d_device = create_d3d_device()?;
    let d3d_context = unsafe { d3d_device.GetImmediateContext()? };
    let dxgi_device = create_dxgi_device(&d3d_device)?;

    // create frame pool
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &dxgi_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        capture_size,
    )?;
    let session = frame_pool.CreateCaptureSession(&capture_item)?;

    // setup sender and reciever for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    frame_pool.FrameArrived(
        &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                sender.send(frame).unwrap();
                Ok(())
            }
        }),
    )?;

    // Start capture
    session.StartCapture()?;

    // wait for frame
    let frame = receiver.recv().unwrap();

    // Copy frame into new texture
    let texture = unsafe {
        let source_texture: ID3D11Texture2D = get_texture_from_surface(&frame.Surface()?)?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        source_texture.GetDesc(&mut desc);
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;

        let copy_texture = {
            let mut texture = None;
            d3d_device.CreateTexture2D(&desc, None, Some(&mut texture))?;
            texture.unwrap()
        };

        d3d_context.CopyResource(Some(&copy_texture.cast()?), Some(&source_texture.cast()?));

        session.Close()?;
        frame_pool.Close()?;

        copy_texture
    };

    // convert texture to bitmap
    let bits = unsafe {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture.cast()?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        d3d_context.Map(
            Some(&resource.clone()),
            0,
            D3D11_MAP_READ,
            0,
            Some(&mut mapped),
        )?;

        // Get a slice of bytes
        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        let bytes_per_pixel = 4;
        let mut bits = vec![0u8; (desc.Width * desc.Height * bytes_per_pixel) as usize];
        for row in 0..desc.Height {
            let data_begin = (row * (desc.Width * bytes_per_pixel)) as usize;
            let data_end = ((row + 1) * (desc.Width * bytes_per_pixel)) as usize;
            let slice_begin = (row * mapped.RowPitch) as usize;
            let slice_end = slice_begin + (desc.Width * bytes_per_pixel) as usize;
            bits[data_begin..data_end].copy_from_slice(&slice[slice_begin..slice_end]);
        }

        d3d_context.Unmap(Some(&resource), 0);

        bits
    };

    // write bitmap to file
    let path = std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let folder = StorageFolder::GetFolderFromPathAsync(&HSTRING::from(&path))?.get()?;
    let file = folder
        .CreateFileAsync(
            &HSTRING::from("screenshot.png"),
            CreationCollisionOption::ReplaceExisting,
        )?
        .get()?;

    {
        let stream = file.OpenAsync(FileAccessMode::ReadWrite)?.get()?;
        let encoder = BitmapEncoder::CreateAsync(BitmapEncoder::PngEncoderId()?, &stream)?.get()?;
        encoder.SetPixelData(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            capture_size.Width as u32,
            capture_size.Height as u32,
            1.0,
            1.0,
            &bits,
        )?;

        encoder.FlushAsync()?.get()?;
    }

    Ok(())
}
