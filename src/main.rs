#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod d3d_device;
mod display;
mod image;
mod logger;
mod support;
mod texture;
mod write_image;

use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

use app::App;
use glium::backend::Facade;
use log::error;

use inputbot::KeybdKey::{self};
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

fn main() -> Result<()> {
    logger::init_fern().unwrap();

    if !GraphicsCaptureSession::IsSupported().unwrap() {
        error!("Graphics capture is not supported.");
        return Ok(());
    }

    let gui = support::init("Screenshot");
    let proxy = gui.event_loop.create_proxy();

    let (sender, receiver) = channel();

    thread::spawn(move || {
        KeybdKey::F13Key.bind(move || {
            let image = take_screenshot();
            sender.send(image).unwrap();
            proxy.send_event(support::AppEvent::Show).unwrap();
        });

        inputbot::handle_input_events();
    });

    let proxy = gui.event_loop.create_proxy();
    let mut app = App::new(receiver, proxy);

    gui.main_loop(move |_, display, renderer, ui| {
        app.render(ui, display.get_context(), renderer.textures());
    });

    Ok(())
}

fn take_screenshot() -> Image {
    // create d3d device for capture item
    let d3d_device = create_d3d_device().unwrap();
    let d3d_context = unsafe { d3d_device.GetImmediateContext().unwrap() };
    let dxgi_device = create_dxgi_device(&d3d_device).unwrap();

    let display = get_display().unwrap();

    // turn display into capture item
    let interop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
    let capture_item: GraphicsCaptureItem =
        unsafe { interop.CreateForMonitor(display.handle) }.unwrap();
    let capture_size = capture_item.Size().unwrap();

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

    let image = unsafe {
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

        let image: Image = Image::from_u8(slice, desc.Width as usize, desc.Height as usize);

        thread::spawn(move || d3d_context.Unmap(Some(&resource), 0));

        image
    };
    image
}
