mod d3d_device;
mod display;
mod image;
mod logger;
mod texture;
mod write_image;

use std::sync::mpsc::channel;
use std::time::SystemTime;

use half::f16;
use log::error;
use rayon::prelude::*;
use windows::core::{ComInterface, IInspectable, Result};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use crate::d3d_device::{create_d3d_device, create_dxgi_device};
use crate::display::get_display;
use crate::image::Image;
use crate::texture::get_texture_from_surface;
use crate::write_image::write_image;

fn main() -> Result<()> {
    let start = SystemTime::now();
    logger::init_fern().unwrap();

    if !GraphicsCaptureSession::IsSupported()? {
        error!("Graphics capture is not supported.");
        return Ok(());
    }

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
    // should set pixel format to match the display's pixel format
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &dxgi_device,
        DirectXPixelFormat::R16G16B16A16Float,
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

    let mut image = unsafe {
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

        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        const BYTES_PER_PIXEL: usize = 8;
        const BYTES_PER_CHANNEL: usize = 2;
        const CHANNELS: usize = 4;

        let mut image: Image = Image::new(desc.Width as usize, desc.Height as usize);

        /* let mut image = (0..desc.Height)
        .into_par_iter()
        .map(|row_index| {
            let mut row = vec![[f16::ZERO; CHANNELS]; desc.Width as usize];
            let slice_begin = (row_index * mapped.RowPitch) as usize;
            let slice_end = slice_begin + (desc.Width * BYTES_PER_PIXEL as u32) as usize;

            let slice = &slice[slice_begin..slice_end];

            for pixel_index in 0..(slice.len() / BYTES_PER_PIXEL) {
                let mut pixel = [f16::ZERO; CHANNELS];

                for channel_index in 0..CHANNELS {
                    let channel_start =
                        (pixel_index * BYTES_PER_PIXEL) + (channel_index * BYTES_PER_CHANNEL);
                    let mut channel = [0u8; BYTES_PER_CHANNEL];

                    for byte_index in 0..BYTES_PER_CHANNEL {
                        channel[byte_index] = slice[channel_start + byte_index];
                    }

                    let pixel_value = f16::from_le_bytes(channel);
                    pixel[channel_index] = pixel_value;
                }

                row[pixel_index] = pixel;
            }
            row
        })
        .collect(); */

        let f16_start = SystemTime::now();
        for row in 0..desc.Height {
            let slice_begin = (row * mapped.RowPitch) as usize;
            let slice_end = slice_begin + (desc.Width * BYTES_PER_PIXEL as u32) as usize;

            let slice = &slice[slice_begin..slice_end];

            for pixel_index in 0..(slice.len() / BYTES_PER_PIXEL) {
                let mut pixel = [f16::ZERO; CHANNELS];

                for channel_index in 0..CHANNELS {
                    let channel_start =
                        (pixel_index * BYTES_PER_PIXEL) + (channel_index * BYTES_PER_CHANNEL);
                    let mut channel = [0u8; BYTES_PER_CHANNEL];

                    for byte_index in 0..BYTES_PER_CHANNEL {
                        channel[byte_index] = slice[channel_start + byte_index];
                    }

                    let pixel_value = f16::from_le_bytes(channel);
                    pixel[channel_index] = pixel_value;
                    if pixel_value > image.max_value {
                        image.max_value = pixel_value;
                    }
                }

                image.rows[row as usize][pixel_index] = pixel;
            }
        }
        let f16_end = SystemTime::now();
        let duration = f16_end.duration_since(f16_start).unwrap();
        println!("f16 took {}s", duration.as_secs_f64());

        d3d_context.Unmap(Some(&resource), 0);

        image
    };

    // good sdr -> sdr values 1.05, 0.5
    let gamma_start = SystemTime::now();
    image.compress_gamma(1.05, 0.5);
    let gamma_end = SystemTime::now();
    let duration = gamma_end.duration_since(gamma_start).unwrap();
    println!("Gamma took {}s", duration.as_secs_f64());

    let width = image.width;
    let height = image.height;

    let u8_start = SystemTime::now();
    let image = image.to_bytes();
    let u8_end = SystemTime::now();
    let duration = u8_end.duration_since(u8_start).unwrap();
    println!("u8 took {}s", duration.as_secs_f64());

    write_image(image, width as u32, height as u32)?;
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    println!("Completed in {}s", duration.as_secs_f64());

    Ok(())
}
