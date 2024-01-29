use std::{fs::File, path::PathBuf};

use chrono::Local;
use image::{codecs::jpeg::JpegEncoder, ColorType, ImageResult};

pub fn save_jpeg(image: &[u8], width: u32, height: u32) -> ImageResult<()> {
    let name = format!("screenshot {}.jpg", Local::now().format("%F %l-%M-%S %P"));
    let path = PathBuf::from(name);

    let file = File::create(path).unwrap();

    let mut encoder = JpegEncoder::new_with_quality(file, 85);
    encoder.encode(image, width, height, ColorType::Rgba8)?;

    Ok(())
}
