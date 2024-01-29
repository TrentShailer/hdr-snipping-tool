mod d3d_device;
mod display;
mod image;
mod logger;
mod texture;
mod write_image;

use std::sync::mpsc::channel;
use std::thread;

use eframe::{EventLoopBuilder, EventLoopBuilderHook};
use egui::load::SizedTexture;
use egui::{Color32, Context, Frame, Key, Pos2, Rect, Slider, TextureOptions, Vec2};
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
use winit::platform::windows::EventLoopBuilderExtWindows;

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

    KeybdKey::F13Key.bind(|| take_screenshot());

    inputbot::handle_input_events();

    Ok(())
}

fn take_screenshot() {
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

    let event_loop_builder: Option<EventLoopBuilderHook> = Some(Box::new(|event_loop_builder| {
        event_loop_builder.with_any_thread(true);
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([image.width as f32, image.height as f32])
            .with_fullscreen(true),
        event_loop_builder,
        ..Default::default()
    };
    eframe::run_native(
        "Test",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(MyApp::new(image))
        }),
    )
    .unwrap();

    // open window
    // let user select area
    // esc to close window
    // modify gamma value
    // unlock alpha value
    // save + copy to clipboard

    // image.save_rgba8();
}

struct MyApp {
    pub image: Image,
    pub texture: Option<egui::TextureHandle>,
}

impl MyApp {
    pub fn new(image: Image) -> Self {
        Self {
            image,
            texture: None,
        }
    }

    fn rebuild_texture(&mut self, ctx: &Context) {
        let handle = ctx.load_texture(
            "screenshot",
            self.image.get_color_image(),
            Default::default(),
        );
        self.texture = Some(handle);
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ctx.input(|i| i.key_pressed(Key::Enter)) {
                    // TODO copy to clipboard
                    self.image.save();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if let Some(texture) = &self.texture {
                    let texture = egui::load::SizedTexture::new(
                        texture.id(),
                        egui::vec2(self.image.width as f32, self.image.height as f32),
                    );
                    ui.image(texture);
                } else {
                    self.rebuild_texture(ctx);
                }
            });
        egui::Window::new("Settings").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.image.gamma, 0.01..=1.0).text("Gamma value"));
            ui.add(Slider::new(&mut self.image.alpha, 0.01..=10.0).text("Alpha value"));
            if ui.button("Auto calculate alpha").clicked() {
                self.image.alpha = self.image.calculate_alpha();
            }
            if ui.button("Apply").clicked() {
                self.rebuild_texture(ctx);
            }
        });
    }
}
