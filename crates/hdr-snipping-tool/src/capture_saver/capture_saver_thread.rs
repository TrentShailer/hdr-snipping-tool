use std::{
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use tracing::{error, info_span};
use vulkan::{HdrImage, Vulkan};

use crate::{
    selection::Selection,
    utilities::failure::{Failure, Ignore},
};

use super::{BlockingCaptureSaver, CaptureSaver};

enum Message {
    Save(HdrImage, f32, Selection),
    Shutdown,
}

pub struct CaptureSaverThread {
    thread: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl CaptureSaverThread {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let (sender, receiver) = channel();

        let thread = thread::Builder::new()
            .name(String::from("Capture Saver"))
            .spawn(move || {
                let _span = info_span!("[Capture Saver]").entered();
                let capture_saver = BlockingCaptureSaver::new(&vulkan);

                loop {
                    // unwrap should never happen, CaptureTaker owns the sender and calls shutdown on drop.
                    let message = receiver.recv().unwrap();

                    match message {
                        Message::Shutdown => break,
                        Message::Save(hdr_image, whitepoint, selection) => {
                            capture_saver.save_capture(hdr_image, whitepoint, selection)
                        }
                    }
                }
            })
            .report_and_panic("Could not start the capture saver thread");

        Self {
            thread: Some(thread),
            sender,
        }
    }
}

impl CaptureSaver for CaptureSaverThread {
    fn save_capture(&self, capture: HdrImage, whitepoint: f32, selection: Selection) {
        self.sender
            .send(Message::Save(capture, whitepoint, selection))
            .report_and_panic("Could not send message to capture saver");
    }
}

impl Drop for CaptureSaverThread {
    fn drop(&mut self) {
        self.sender.send(Message::Shutdown).ignore();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Joining Capture Saver thread returned an error");
            }
        }
    }
}
