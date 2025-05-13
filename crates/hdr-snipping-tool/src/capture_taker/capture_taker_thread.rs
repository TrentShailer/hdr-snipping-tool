use core::time::Duration;
use std::{
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use tracing::{error, info_span};
use vulkan::Vulkan;
use windows_capture_provider::WindowsCapture;
use winit::event_loop::EventLoopProxy;

use crate::{
    application_event_loop::Event,
    utilities::failure::{Failure, Ignore},
};

use super::{BlockingCaptureTaker, CaptureTaker};

enum Message {
    Shutdown,
    TakeCapture(EventLoopProxy<Event>),
    CleanupWindowsCapture(WindowsCapture),
    RefreshCache,
}

pub struct CaptureTakerThread {
    thread: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl CaptureTakerThread {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let (sender, receiver) = channel();

        let thread = thread::Builder::new()
            .name(String::from("Capture Taker"))
            .spawn(move || {
                let _span = info_span!("[Capture Taker]").entered();
                let mut capture_taker = BlockingCaptureTaker::new(&vulkan);

                loop {
                    // Unwrap should never happen, CaptureTaker owns the sender and calls shutdown on drop.
                    let message = receiver.recv().unwrap();

                    match message {
                        Message::Shutdown => break,
                        Message::RefreshCache => capture_taker.refresh_cache(),
                        Message::TakeCapture(proxy) => capture_taker.take_capture(proxy),
                        Message::CleanupWindowsCapture(capture) => {
                            capture_taker.cleanup_windows_capture(capture)
                        }
                    }
                }
            })
            .report_and_panic("Could not start the capture taker thread");

        // Start the thread to request cache refresh
        {
            let sender = sender.clone();
            thread::Builder::new()
                .name("Refresh Cache".into())
                .spawn(move || {
                    let _span = info_span!("[Refresh Cache]").entered();

                    loop {
                        if sender.send(Message::RefreshCache).is_err() {
                            break;
                        }

                        thread::sleep(Duration::from_secs(60 * 10));
                    }
                })
                .report_and_panic("Could not start the cache thread");
        };

        Self {
            thread: Some(thread),
            sender,
        }
    }
}

impl CaptureTaker for CaptureTakerThread {
    fn cleanup_windows_capture(&self, capture: WindowsCapture) {
        self.sender
            .send(Message::CleanupWindowsCapture(capture))
            .report_and_panic("Could not send message to capture taker");
    }

    fn take_capture(&mut self, proxy: EventLoopProxy<Event>) {
        self.sender
            .send(Message::TakeCapture(proxy))
            .report_and_panic("Could not send message to capture taker");
    }

    fn refresh_cache(&mut self) {
        self.sender
            .send(Message::RefreshCache)
            .report_and_panic("Could not send message to capture taker");
    }
}

impl Drop for CaptureTakerThread {
    fn drop(&mut self) {
        self.sender.send(Message::Shutdown).ignore();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Joining Capture Taker thread returned an error");
            }
        }
    }
}
