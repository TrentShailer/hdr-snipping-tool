use std::{
    fs::{self},
    path::PathBuf,
};

use chrono::Local;
use image::{codecs::jpeg::JpegEncoder, GenericImageView, ImageBuffer, Rgba};

pub fn save_jpeg(
    image: &[u8],
    selection: [[u32; 2]; 2],
    width: u32,
    height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image.to_owned())
            .unwrap()
            .view(
                selection[0][0],
                selection[0][1],
                selection[1][0],
                selection[1][1],
            )
            .to_image();

    let name = format!("screenshot {}.jpg", Local::now().format("%F %l-%M-%S %P"));
    let path = PathBuf::from(name);

    let mut jpeg_buffer = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut jpeg_buffer, 90);
    img.write_with_encoder(encoder).unwrap();
    fs::write(path, &jpeg_buffer).unwrap();

    img
}
