extern crate alloc;

use alloc::sync::Arc;
use std::{
    sync::mpsc::{Sender, channel},
    thread::{self, JoinHandle},
};
use tracing::debug;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use testing::setup_logger;
use vulkan::{HdrImage, HdrScanner, Renderer, RendererState, Vulkan};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::WindowEvent,
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
        let (windows_capture, monitor) = {
            let direct_x = DirectX::new().unwrap();
            let mut cache = CaptureItemCache::new();

            let monitor = Monitor::get_hovered_monitor(&direct_x).unwrap().unwrap();
            let capture_item = cache.get_capture_item(monitor.handle.0).unwrap();

            let (capture, resources) =
                WindowsCapture::take_capture(&direct_x, &capture_item).unwrap();

            debug!("Capture Size: {:?}", capture.size);

            unsafe { resources.destroy(&direct_x).unwrap() };

            (capture, monitor)
        };

        let window = {
            let window_attributes = Window::default_attributes()
                .with_title("Renderer Testing")
                .with_active(false)
                .with_visible(false);

            event_loop.create_window(window_attributes).unwrap()
        };

        let vulkan = Arc::new(
            Vulkan::new(
                true,
                std::env::current_exe().unwrap().parent().unwrap(),
                Some(window.display_handle().unwrap().as_raw()),
            )
            .unwrap(),
        );

        let (render_state, render_sender, render_thread) = {
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

            (render_state, render_sender, render_thread)
        };

        let capture = unsafe {
            HdrImage::import_windows_capture(
                &vulkan,
                windows_capture.size,
                windows_capture.handle.0.0 as isize,
            )
            .unwrap()
        };

        // Update state using the capture
        {
            let mut hdr_scanner = HdrScanner::new(Arc::clone(&vulkan)).unwrap();
            let maximum = unsafe { hdr_scanner.scan(capture).unwrap() };

            let mut render_state = render_state.lock();
            render_state.capture = Some(capture);
            render_state.whitepoint = monitor.sdr_white;
            render_state.max_brightness = if maximum <= monitor.sdr_white {
                monitor.sdr_white
            } else {
                monitor.max_brightness
            };

            drop(render_state);
        }

        window.set_visible(true);
        window.set_maximized(true);

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
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else {
            return;
        };

        if event == WindowEvent::Destroyed && app.window.id() == window_id {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(_) => app
                .render_sender
                .send(RenderMessage::RequestResize)
                .unwrap(),

            WindowEvent::RedrawRequested => app.render_sender.send(RenderMessage::Render).unwrap(),

            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let mut state = app.render_state.lock();
                state.mouse_position = position.into();
                state.selection = [
                    PhysicalPosition::new(position.x - 400.0, position.y - 400.0).into(),
                    PhysicalPosition::new(position.x + 400.0, position.y + 400.0).into(),
                ];
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

    let thread = thread::spawn(move || {
        loop {
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
        }
    });

    (sender, thread)
}
