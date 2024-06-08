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
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;

use crate::message_box::display_message;

use super::App;

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to copy texture to CPU:\n{0}")]
    CpuCopy(#[from] vulkan_backend::texture::copy_to_cpu::Error),

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
    pub fn save_capture(&mut self) {
        if let Err(e) = self.save_capture_inner() {
            log::error!("{e}");
            match e {
                Error::ClipboardInstance(_) => display_message("We encountered an error while getting a clipboard instance.\nMore details are in the logs.", MB_ICONERROR),
                Error::ClipboardSave(_) => display_message("We encountered an error while saving the capture to your clipboard.\nMore details are inthe logs.", MB_ICONERROR),
                Error::CpuCopy(_) => display_message("We encountered an error while copying the capture to your CPU.\nMore details are in the logs.", MB_ICONERROR),
                Error::CreateFile(_) => display_message("We encountered an error while creating the file to save the capture in.\nMore details are in the logs.", MB_ICONERROR),
                Error::WriteFile(_) => display_message("We encountered an error while writing the capture to its file.\nMore details are in the logs", MB_ICONERROR)
            }
            std::process::exit(-1);
        }

        let backend = match self.backend.as_mut() {
            Some(v) => v,
            None => return,
        };

        backend.renderer.renderpass_capture.capture = None;
        backend.renderer.renderpass_capture.capture_ds = None;
        backend.tonemapper.clear();

        let window = match self.window.as_ref() {
            Some(v) => v,
            None => return,
        };
        window.set_visible(false);
    }

    fn save_capture_inner(&mut self) -> Result<(), Error> {
        let backend = match self.backend.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };
        let vulkan_instance = match self.vulkan_instance.as_mut() {
            Some(v) => v,
            None => return Ok(()),
        };

        let mut texture = match backend.renderer.renderpass_capture.capture.take() {
            Some(v) => v,
            None => return Ok(()),
        };

        let raw_capture = texture.copy_to_cpu(&vulkan_instance)?;

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(texture.size.width, texture.size.height, raw_capture).unwrap(); // Unwrap should be safe

        let (selection_pos, selection_size) = self.selection.as_pos_size();

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = img
            .view(
                selection_pos.x,
                selection_pos.y,
                selection_size.width,
                selection_size.height,
            )
            .to_image();

        let name = format!("screenshot {}.png", Local::now().format("%F %H-%M-%S"));
        let path = PathBuf::from(name);

        let file = File::create(path).map_err(Error::CreateFile)?;
        let mut buffer = BufWriter::new(file);

        let encoder = PngEncoder::new(&mut buffer);

        img.write_with_encoder(encoder)?;

        let mut clipboard = Clipboard::new().map_err(Error::ClipboardInstance)?;
        clipboard
            .set_image(ImageData {
                width: selection_size.width as usize,
                height: selection_size.height as usize,
                bytes: Cow::Borrowed(img.as_raw()),
            })
            .map_err(Error::ClipboardSave)?;
        Ok(())
    }
}
