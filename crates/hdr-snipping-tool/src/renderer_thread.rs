use std::{
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use parking_lot::Mutex;
use tracing::{error, info_span};
use vulkan::{HdrImage, RendererState, Vulkan};
use winit::{
    dpi::PhysicalPosition,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::{
    selection::Selection,
    utilities::failure::{Failure, Ignore},
};

#[derive(PartialEq, Eq)]
enum Message {
    Render,
    Resize,
    Shutdown,
}

pub struct RendererThread {
    // Option allows for joining the thread which requires ownership.
    thread: Option<JoinHandle<()>>,
    sender: Sender<Message>,
    state: Arc<Mutex<RendererState>>,
}

impl RendererThread {
    pub fn new(vulkan: Arc<Vulkan>, window: &Window) -> Self {
        let (sender, receiver) = channel();

        let state = Arc::new(Mutex::new(RendererState::default()));

        let mut renderer = unsafe {
            vulkan::Renderer::new(
                vulkan,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                Arc::clone(&state),
            )
            .report_and_panic("Could not create the renderer")
        };

        // Start the thread to handle taking the capture
        let thread = thread::Builder::new()
            .name("Capture Taker".into())
            .spawn(move || {
                let _span = info_span!("[Renderer]").entered();

                loop {
                    // unwrap should never happen, CaptureTaker owns the sender and calls shutdown on drop.
                    let message = receiver.recv().unwrap();

                    // Pump backed up events
                    let mut messages = vec![message];
                    while let Ok(message) = receiver.try_recv() {
                        if !messages.contains(&message) {
                            messages.push(message);
                        }
                    }

                    // Handle messages
                    if messages.contains(&Message::Shutdown) {
                        break;
                    }

                    if messages.contains(&Message::Resize) {
                        renderer.request_resize();
                    }

                    if messages.contains(&Message::Render) {
                        unsafe { renderer.render() }
                            .report_and_panic("Encountered an error while rendering");
                    }
                }

                drop(renderer);
            })
            .report_and_panic("Could not start the capture taker thread");

        Self {
            thread: Some(thread),
            sender,
            state,
        }
    }

    pub fn resize(&self) {
        self.sender
            .send(Message::Resize)
            .report_and_panic("Could not send message to renderer");
    }

    pub fn render(&self) {
        self.sender
            .send(Message::Render)
            .report_and_panic("Could not send message to renderer");
    }

    pub fn set_mouse_position(&mut self, position: PhysicalPosition<f32>) {
        let mut state = self.state.lock();
        state.mouse_position = position.into();
    }

    pub fn set_selection(&mut self, selection: Selection) {
        let mut state = self.state.lock();
        state.selection = [selection.start.into(), selection.end.into()];
    }

    pub fn set_hdr_capture(&mut self, hdr_capture: Option<HdrImage>) {
        let mut state = self.state.lock();
        state.capture = hdr_capture;
    }

    pub fn set_whitepoint(&mut self, whitepoint: f32) {
        let mut state = self.state.lock();
        state.whitepoint = whitepoint;
    }

    pub fn set_max_brightness(&mut self, max_brightness: f32) {
        let mut state = self.state.lock();
        state.max_brightness = max_brightness;
    }
}

impl Drop for RendererThread {
    fn drop(&mut self) {
        self.sender.send(Message::Shutdown).ignore();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Joining Render thread returned an error.");
            };
        }
    }
}
