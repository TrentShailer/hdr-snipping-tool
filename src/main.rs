mod d3d_device;
mod display;
mod image;
mod logger;
mod texture;
mod write_image;

use std::sync::mpsc::channel;
use std::time::SystemTime;

use ::image::{ColorType, ImageFormat};
use log::error;

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
use crate::write_image::save_jpeg;

fn main() -> Result<()> {
    logger::init_fern().unwrap();

    if !GraphicsCaptureSession::IsSupported().unwrap() {
        error!("Graphics capture is not supported.");
        return Ok(());
    }

    let display = get_display().unwrap();

    // turn display into capture item
    let interop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
    let capture_item: GraphicsCaptureItem =
        unsafe { interop.CreateForMonitor(display.handle) }.unwrap();
    let capture_size = capture_item.Size().unwrap();

    // create d3d device for capture item
    let d3d_device = create_d3d_device().unwrap();
    let d3d_context = unsafe { d3d_device.GetImmediateContext().unwrap() };
    let dxgi_device = create_dxgi_device(&d3d_device).unwrap();

    // create frame pool
    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &dxgi_device,
        DirectXPixelFormat::R16G16B16A16Float,
        1,
        capture_size,
    )
    .unwrap();
    let session = frame_pool.CreateCaptureSession(&capture_item).unwrap();

    // setup sender and reciever for frames
    let (sender, receiver) = channel();

    // handle frames arriving
    frame_pool
        .FrameArrived(
            &TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                move |frame_pool, _| {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame().unwrap();
                    sender.send(frame).unwrap();
                    Ok(())
                }
            }),
        )
        .unwrap();

    // Start capture
    session.StartCapture().unwrap();

    // wait for frame
    let frame = receiver.recv().unwrap();
    let texture_start = SystemTime::now();
    // Copy frame into new texture
    let texture = unsafe {
        let source_texture: ID3D11Texture2D =
            get_texture_from_surface(&frame.Surface().unwrap()).unwrap();
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
                .unwrap();
            texture.unwrap()
        };

        d3d_context.CopyResource(
            Some(&copy_texture.cast().unwrap()),
            Some(&source_texture.cast().unwrap()),
        );

        session.Close().unwrap();
        frame_pool.Close().unwrap();

        copy_texture
    };
    let texture_end = SystemTime::now();
    let duration = texture_end.duration_since(texture_start).unwrap();
    println!("texture in {}s", duration.as_secs_f64());

    let mut image = unsafe {
        let slice_start = SystemTime::now();

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        texture.GetDesc(&mut desc as *mut _);

        let resource: ID3D11Resource = texture.cast().unwrap();
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        d3d_context
            .Map(
                Some(&resource.clone()),
                0,
                D3D11_MAP_READ,
                0,
                Some(&mut mapped),
            )
            .unwrap();

        let slice: &[u8] = {
            std::slice::from_raw_parts(
                mapped.pData as *const _,
                (desc.Height * mapped.RowPitch) as usize,
            )
        };

        let slice_end = SystemTime::now();
        let duration = slice_end.duration_since(slice_start).unwrap();
        println!("slice in {}s", duration.as_secs_f64());

        let f32_start = SystemTime::now();
        let image: Image = Image::from_u8(slice, desc.Width as usize, desc.Height as usize);
        let f32_end = SystemTime::now();
        let duration = f32_end.duration_since(f32_start).unwrap();
        println!("f32 took {}s", duration.as_secs_f64());

        // TODO could move this to separate thread
        d3d_context.Unmap(Some(&resource), 0);

        image
    };

    // good sdr -> sdr values 1.00, 0.470

    let gamma_start = SystemTime::now();
    image.compress_gamma(1.00, 0.47);
    let gamma_end = SystemTime::now();
    let duration = gamma_end.duration_since(gamma_start).unwrap();
    println!("Gamma took {}s", duration.as_secs_f64());

    let width = image.width;
    let height = image.height;

    let image = image.into_rgba8();

    let write_start = SystemTime::now();
    save_jpeg(&image, width as u32, height as u32).unwrap();
    let write_end = SystemTime::now();
    let duration = write_end.duration_since(write_start).unwrap();
    println!("Write took {}s", duration.as_secs_f64());

    Ok(())
}
