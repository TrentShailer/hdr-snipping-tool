extern crate alloc;

use alloc::sync::Arc;
use std::{
    sync::mpsc::{channel, Sender},
    thread::{self, JoinHandle},
};
use tracing::debug;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

use ash_helper::VulkanContext;
use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use testing::setup_logger;
use vulkan::{HdrImage, Renderer, RendererState, Vulkan};
use winit::{
    application::ApplicationHandler, dpi::PhysicalPosition, event_loop::EventLoop, window::Window,
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
    vulkan: Option<Arc<Vulkan>>,
    render_thread: Option<JoinHandle<()>>,
    render_sender: Option<Sender<RenderMessage>>,
    render_state: Option<Arc<Mutex<RendererState>>>,
    window: Option<Window>,
    capture: Option<HdrImage>,
}

impl App {
    pub fn new() -> Self {
        Self {
            vulkan: None,
            render_sender: None,
            render_thread: None,
            window: None,
            render_state: None,
            capture: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Renderer Testing")
            .with_active(false)
            .with_visible(false);

        let window = event_loop.create_window(window_attributes).unwrap();

        let vulkan = unsafe {
            Arc::new(Vulkan::new(true, Some(window.display_handle().unwrap().as_raw())).unwrap())
        };
        let direct_x = DirectX::new().unwrap();
        let mut cache = CaptureItemCache::new();

        let renderer = unsafe {
            Renderer::new(
                vulkan.clone(),
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
            )
            .unwrap()
        };

        self.render_state = Some(renderer.state.clone());

        let (windows_capture, capture) = {
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

        let (render_sender, render_thread) = render_thread(renderer);

        let mut render_state = self.render_state.as_ref().unwrap().lock();
        render_state.capture = Some(capture);
        render_state.whitepoint = windows_capture.monitor.sdr_white;
        drop(render_state);

        window.set_visible(true);

        self.capture = Some(capture);

        self.window = Some(window);
        self.vulkan = Some(vulkan);
        self.render_sender = Some(render_sender);
        self.render_thread = Some(render_thread);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if self.window.is_none() {
            return;
        };

        if event == winit::event::WindowEvent::Destroyed
            && self.window.as_ref().unwrap().id() == window_id
        {
            self.render_sender
                .as_ref()
                .unwrap()
                .send(RenderMessage::Shutdown)
                .unwrap();

            self.render_thread.take().unwrap().join().unwrap();

            let capture = self.capture.take().unwrap();
            let vulkan = self.vulkan.take().unwrap();
            unsafe {
                vulkan.device().device_wait_idle().unwrap();
                capture.destroy(&vulkan);
            }

            event_loop.exit();
            return;
        }

        match event {
            winit::event::WindowEvent::CloseRequested => {
                self.render_sender
                    .as_ref()
                    .unwrap()
                    .send(RenderMessage::Shutdown)
                    .unwrap();

                self.render_thread.take().unwrap().join().unwrap();

                let capture = self.capture.take().unwrap();
                let vulkan = self.vulkan.take().unwrap();
                unsafe {
                    vulkan.device().device_wait_idle().unwrap();
                    capture.destroy(&vulkan);
                }

                event_loop.exit();
            }

            winit::event::WindowEvent::Resized(size) => {
                let width = size.width as f64;
                let height = size.height as f64;

                let mut state = self.render_state.as_ref().unwrap().lock();
                state.selection = [
                    PhysicalPosition::new(width / 2.0, height / 2.0).into(),
                    PhysicalPosition::new(width / 2.0 - width / 4.0, height / 2.0 - height / 4.0)
                        .into(),
                ];
                drop(state);

                self.render_sender
                    .as_ref()
                    .unwrap()
                    .send(RenderMessage::RequestResize)
                    .unwrap();
            }

            winit::event::WindowEvent::RedrawRequested => {
                self.render_sender
                    .as_ref()
                    .unwrap()
                    .send(RenderMessage::Render)
                    .unwrap();
            }

            winit::event::WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let mut state = self.render_state.as_ref().unwrap().lock();
                state.mouse_position = position.into();
                drop(state);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            return;
        };
        self.window.as_ref().unwrap().request_redraw();
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
