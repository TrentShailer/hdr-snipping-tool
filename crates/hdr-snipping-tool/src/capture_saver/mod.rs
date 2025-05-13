use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use chrono::Local;
use image::{GenericImageView, ImageBuffer, ImageFormat, Rgba};
use tracing::{info, warn};
use utilities::DebugTime;
use vulkan::{HdrImage, HdrToSdrTonemapper, Vulkan};

use crate::{
    screenshot_dir,
    selection::Selection,
    utilities::failure::{Failure, report},
};

pub use capture_saver_thread::CaptureSaverThread;

mod capture_saver_thread;

pub trait CaptureSaver {
    fn save_capture(&self, capture: HdrImage, whitepoint: f32, selection: Selection);
}

pub struct BlockingCaptureSaver<'vulkan> {
    vulkan: &'vulkan Vulkan,
    tonemapper: HdrToSdrTonemapper<'vulkan>,
}

impl<'vulkan> BlockingCaptureSaver<'vulkan> {
    pub fn new(vulkan: &'vulkan Vulkan) -> Self {
        let tonemapper =
            HdrToSdrTonemapper::new(vulkan).report_and_panic("Could not create the tonemapper");

        Self { vulkan, tonemapper }
    }
}

impl CaptureSaver for BlockingCaptureSaver<'_> {
    fn save_capture(&self, capture: HdrImage, whitepoint: f32, selection: Selection) {
        // Tonemap the image
        let sdr_image = match unsafe { self.tonemapper.tonemap(capture, whitepoint) } {
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
        let bytes = match unsafe { sdr_image.copy_to_cpu(self.vulkan) } {
            Ok(bytes) => bytes,
            Err(e) => {
                report(
                    e,
                    "Could not save the screenshot.\nEncountered an error while copying the screenshot to CPU Memory",
                );
                return;
            }
        };

        // Destroy SDR image
        unsafe { sdr_image.destroy(self.vulkan) };

        let selection_position = selection.position();
        let selection_size = selection.size();

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
            let _timing = DebugTime::start("Saving to file");
            let name = format!("Screenshot {}.png", Local::now().format("%F %H%M%S"));
            let path = screenshot_dir().join(name);

            match img.save_with_format(&path, ImageFormat::Png) {
                Ok(_) => info!("Saved screenshot to file"),
                Err(e) => report(e, "Could not save the screenshot file"),
            }
        }

        // Save to clipboard
        {
            let _timing = DebugTime::start("Saving to clipboard");

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

            match save_result {
                Ok(_) => info!("Saved screenshot to clipboard"),
                Err(e) => report(e, "Could not save the screenshot to the clipboard"),
            }
        }
    }
}
