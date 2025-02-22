use std::{
    borrow::Cow,
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread::{self, JoinHandle},
};

use arboard::{Clipboard, ImageData};
use chrono::Local;
use image::{GenericImageView, ImageBuffer, ImageFormat, Rgba};
use tracing::{error, info, info_span, warn};
use vulkan::{HdrToSdrTonemapper, Vulkan};

use crate::{
    screenshot_dir,
    utilities::failure::{Failure, report},
};

use super::capture::Capture;

#[allow(clippy::large_enum_variant)]
enum Message {
    Save(Capture),
    Shutdown,
}

pub struct CaptureSaver {
    // Option allows for joining the thread which requires ownership.
    thread: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl CaptureSaver {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let (sender, receiver) = channel();

        // Start the thread to handle taking the capture
        let thread = thread::Builder::new()
            .name("Capture Saver".into())
            .spawn(move || {
                let _span = info_span!("[Capture Saver]").entered();
                let inner = InnerCaptureSaver::new(vulkan);

                loop {
                    // unwrap should never happen, CaptureTaker owns the sender and calls shutdown on drop.
                    let message = receiver.recv().unwrap();

                    match message {
                        Message::Shutdown => break,
                        Message::Save(capture) => inner.save(capture),
                    }
                }

                drop(inner);
            })
            .report_and_panic("Could not start the capture saver thread");

        Self {
            thread: Some(thread),
            sender,
        }
    }

    pub fn save(&self, capture: Capture) -> Result<(), ()> {
        if let Err(e) = self.sender.send(Message::Save(capture)) {
            error!("Failed to send message to capture saver: {e}");
            return Err(());
        }

        Ok(())
    }
}

impl Drop for CaptureSaver {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Joining Capture Saver thread returned an error.");
            };
        }
    }
}

pub struct InnerCaptureSaver {
    vulkan: Arc<Vulkan>,
    tonemapper: HdrToSdrTonemapper,
}

impl InnerCaptureSaver {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let tonemapper = HdrToSdrTonemapper::new(Arc::clone(&vulkan))
            .report_and_panic("Could not create the tonemapper");

        Self { vulkan, tonemapper }
    }

    pub fn save(&self, capture: Capture) {
        let Some(hdr_image) = capture.hdr_capture else {
            return;
        };

        // Tonemap the image
        let sdr_image = match unsafe { self.tonemapper.tonemap(hdr_image, capture.whitepoint) } {
            Ok(sdr_image) => sdr_image,
            Err(e) => {
                report(
                    e,
                    "Could not save the screenshot.\nEncountered an error while tonemapping",
                );
                return;
            }
        };

        // Copy the image to CPU Memory
        let bytes = match unsafe { sdr_image.copy_to_cpu(&self.vulkan) } {
            Ok(bytes) => bytes,
            Err(e) => {
                report(
                    e,
                    "Could not save the screenshot.\nEncountered an error while copying the screenshot to CPU Memory",
                );
                return;
            }
        };

        // Destroy sdr image
        unsafe { sdr_image.destroy(&self.vulkan) };

        let selection_position = capture.selection.position();
        let selection_size = capture.selection.size();

        // Create selection view
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(sdr_image.extent.width, sdr_image.extent.height, bytes)
                .unwrap()
                .view(
                    selection_position.x as u32,
                    selection_position.y as u32,
                    selection_size.width as u32,
                    selection_size.height as u32,
                )
                .to_image();

        // Save to file
        {
            let name = format!("Screenshot {}.png", Local::now().format("%F %H%M%S"));
            let path = screenshot_dir().join(name);

            if let Err(e) = img.save_with_format(&path, ImageFormat::Png) {
                report(
                    e,
                    "Could not save the screenshot.\nCould not save the screenshot file",
                );
            }
        }

        // Save to clipboard
        {
            let mut clipboard = match Clipboard::new() {
                Ok(clipboard) => clipboard,
                Err(e) => {
                    warn!("Platform does not support clipboard: {e}");
                    return;
                }
            };

            let save_result = clipboard.set_image(ImageData {
                width: selection_size.width as usize,
                height: selection_size.height as usize,
                bytes: Cow::Borrowed(img.as_raw()),
            });

            if let Err(e) = save_result {
                report(e, "Could not save the screenshot to the clipboard");

                #[allow(clippy::needless_return)]
                return;
            }
        }

        info!("Saved screenshot");
        drop(capture);
    }
}
