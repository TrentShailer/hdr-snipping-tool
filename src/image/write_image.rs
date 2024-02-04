use std::{
    fs::{self},
    path::PathBuf,
};

use chrono::Local;
use glium::glutin::dpi::{PhysicalPosition, PhysicalSize};
use image::{codecs::png::PngEncoder, GenericImageView, ImageBuffer, Rgba};

pub fn save_image(
    image: &[u8],
    selection_pos: PhysicalPosition<u32>,
    selection_size: PhysicalSize<u32>,
    size: PhysicalSize<u32>,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(size.width, size.height, image.to_owned())
            .unwrap()
            .view(
                selection_pos.x,
                selection_pos.y,
                selection_size.width,
                selection_size.height,
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
