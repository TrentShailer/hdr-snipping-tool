use std::{
    fs::{self},
    path::PathBuf,
};

use chrono::Local;
use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, Rgba};

pub fn save_jpeg(
    image: &[u8],
    selection_pos: [u32; 2],
    selection_size: [u32; 2],
    width: u32,
    height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, image.to_owned())
            .unwrap()
            .view(
                selection_pos[0],
                selection_pos[1],
                selection_size[0],
                selection_size[1],
            )
            .to_image();

    let name = format!("screenshot {}.png", Local::now().format("%F %l-%M-%S %P"));
    let path = PathBuf::from(name);

    let mut buffer = Vec::new();
    let encoder = PngEncoder::new(&mut buffer);
    img.write_with_encoder(encoder).unwrap();
    fs::write(path, &buffer).unwrap();

    img
}
