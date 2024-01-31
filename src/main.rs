#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod capture;
mod gui;
mod image;
mod logger;

use std::sync::mpsc::channel;
use std::thread;

use anyhow::{bail, Context};
use app::App;
use capture::get_capture;

use glium::glutin;
use glium::glutin::window::WindowBuilder;
use log::error;

use inputbot::KeybdKey::{self};
use windows::Graphics::Capture::GraphicsCaptureSession;

fn main() {
    logger::init_fern().unwrap();

    if let Err(e) = run() {
        error!("{:?}", e);
    }
}

fn run() -> anyhow::Result<()> {
    if !GraphicsCaptureSession::IsSupported().unwrap() {
        bail!("Graphics capture is not supported.");
    }

    let window = WindowBuilder::new()
        .with_title("Screenshot")
        .with_fullscreen(Some(glutin::window::Fullscreen::Borderless(None)))
        .with_visible(false);

    let gui = gui::init(window).context("Failed to create gui.")?;
    let proxy = gui.event_loop.create_proxy();

    let (sender, receiver) = channel();

    thread::spawn(move || {
        KeybdKey::F13Key.bind(move || {
            let (image, display) = get_capture();
            sender.send((image, display)).unwrap();
            proxy.send_event(gui::AppEvent::Show).unwrap();
        });

        inputbot::handle_input_events();
    });

    let proxy = gui.event_loop.create_proxy();
    let mut app = App::new(receiver, proxy);

    gui.main_loop(move |_, display, renderer, ui| {
        app.render(ui, display, renderer.textures());
    });

    Ok(())
}
