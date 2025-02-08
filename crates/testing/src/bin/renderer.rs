extern crate alloc;

use alloc::sync::Arc;
use std::{
    sync::mpsc::{channel, Sender},
    thread::{self, JoinHandle},
};
use tracing::debug;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use testing::setup_logger;
use vulkan::{HdrImage, Renderer, RendererState, Vulkan};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

fn main() {
    let _guards = setup_logger().unwrap();

    // Create event loop
    let event_loop: EventLoop<()> = EventLoop::with_user_event().build().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    app: Option<ActiveApp>,
}

struct ActiveApp {
    vulkan: Arc<Vulkan>,
    render_thread: Option<JoinHandle<()>>,
    render_sender: Sender<RenderMessage>,
    render_state: Arc<Mutex<RendererState>>,
    window: Window,
    capture: HdrImage,
}

impl ActiveApp {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window_attributes = Window::default_attributes()
            .with_title("Renderer Testing")
            .with_active(false)
            .with_visible(false);

        let window = event_loop.create_window(window_attributes).unwrap();

        let vulkan = unsafe {
            Arc::new(Vulkan::new(true, Some(window.display_handle().unwrap().as_raw())).unwrap())
        };

        let (windows_capture, capture) = {
            let direct_x = DirectX::new().unwrap();
            let mut cache = CaptureItemCache::new();

            let monitor = Monitor::get_hovered_monitor(&direct_x).unwrap().unwrap();
            let capture_item = cache.get_capture_item(monitor).unwrap();

            let (capture, resources) =
                WindowsCapture::take_capture(&direct_x, monitor, &capture_item).unwrap();

            debug!("Capture Size: {:?}", capture.size);

            let hdr_capture = unsafe {
                HdrImage::import_windows_capture(
                    &vulkan,
                    capture.size,
                    capture.handle.0 .0 as isize,
                )
                .unwrap()
            };

            resources.destroy(&direct_x).unwrap();

            (capture, hdr_capture)
        };

        let renderer = unsafe {
            Renderer::new(
                vulkan.clone(),
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
            )
            .unwrap()
        };

        let render_state = renderer.state.clone();

        let (render_sender, render_thread) = render_thread(renderer);

        {
            let mut render_state = render_state.lock();
            render_state.capture = Some(capture);
            render_state.whitepoint = windows_capture.monitor.sdr_white;
            drop(render_state);
        }

        window.set_visible(true);

        Self {
            vulkan,
            render_thread: Some(render_thread),
            render_sender,
            render_state,
            window,
            capture,
        }
    }
}

impl Drop for ActiveApp {
    fn drop(&mut self) {
        unsafe {
            self.capture.destroy(&self.vulkan);
            self.render_sender.send(RenderMessage::Shutdown).unwrap();
            self.render_thread.take().unwrap().join().unwrap();
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self { app: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let app = ActiveApp::new(event_loop);

        self.app = Some(app);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.app.take();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else {
            return;
        };

        if event == winit::event::WindowEvent::Destroyed && app.window.id() == window_id {
            event_loop.exit();
            return;
        }

        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),

            winit::event::WindowEvent::Resized(size) => {
                let width = size.width as f64;
                let height = size.height as f64;

                let mut state = app.render_state.lock();
                state.selection = [
                    PhysicalPosition::new(width / 2.0, height / 2.0).into(),
                    PhysicalPosition::new(width / 2.0 - width / 4.0, height / 2.0 - height / 4.0)
                        .into(),
                ];
                drop(state);

                app.render_sender
                    .send(RenderMessage::RequestResize)
                    .unwrap();
            }

            winit::event::WindowEvent::RedrawRequested => {
                app.render_sender.send(RenderMessage::Render).unwrap();
            }

            winit::event::WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let mut state = app.render_state.lock();
                state.mouse_position = position.into();
                drop(state);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else {
            return;
        };
        app.window.request_redraw();
    }
}

#[derive(PartialEq, Eq)]
enum RenderMessage {
    Render,
    Shutdown,
    RequestResize,
}

fn render_thread(mut renderer: Renderer) -> (Sender<RenderMessage>, JoinHandle<()>) {
    let (sender, receiver) = channel::<RenderMessage>();

    let thread = thread::spawn(move || loop {
        let mut messages = vec![];

        // Pump all of the messages received while rendering
        while let Ok(message) = receiver.try_recv() {
            if !messages.contains(&message) {
                messages.push(message);
            }
        }

        if messages.contains(&RenderMessage::Shutdown) {
            return;
        }

        if messages.contains(&RenderMessage::RequestResize) {
            renderer.request_resize();
        }

        if messages.contains(&RenderMessage::Render) {
            unsafe { renderer.render().unwrap() };
        }
    });

    (sender, thread)
}
