use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufWriter},
    path::PathBuf,
};

use arboard::{Clipboard, ImageData};
use chrono::Local;
use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, ImageError, Rgba};
use thiserror::Error;

use super::{ActiveCapture, App};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to copy texture to CPU:\n{0}")]
    VecCopy(#[from] vulkan_instance::texture::copy_to_vec::Error),

    #[error("Failed to create file for capture:\n{0}")]
    CreateFile(#[source] io::Error),

    #[error("Failed to write capture to file:\n{0}")]
    WriteFile(#[from] ImageError),

    #[error("Failed to get an clipboard instance:\n{0}")]
    ClipboardInstance(#[source] arboard::Error),

    #[error("Failed to save the capture in the clipboard:\n{0}")]
    ClipboardSave(#[source] arboard::Error),
}

impl App {
    pub fn save_capture(&mut self) -> Result<(), Error> {
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        let ActiveCapture { texture, .. } = match self.capture.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        // Create image buffer
        let raw_capture = texture.copy_to_vec(&app.vulkan_instance)?;
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(texture.size.width, texture.size.height, raw_capture).unwrap(); // Unwrap is safe as long as width and height are correct

        // Get selection view
        let (selection_pos, selection_size) = self.selection.as_pos_size();
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = img
            .view(
                selection_pos.x,
                selection_pos.y,
                selection_size.width,
                selection_size.height,
            )
            .to_image();

        // Write image to file
        let name = format!("screenshot {}.png", Local::now().format("%F %H-%M-%S"));
        let path = PathBuf::from(name);
        let file = File::create(path).map_err(Error::CreateFile)?;
        let mut buffer = BufWriter::new(file);
        let encoder = PngEncoder::new(&mut buffer);
        img.write_with_encoder(encoder)?;

        // Set clipboard
        let mut clipboard = Clipboard::new().map_err(Error::ClipboardInstance)?;
        clipboard
            .set_image(ImageData {
                width: selection_size.width as usize,
                height: selection_size.height as usize,
                bytes: Cow::Borrowed(img.as_raw()),
            })
            .map_err(Error::ClipboardSave)?;

        self.clear_capture();

        Ok(())
    }
}
