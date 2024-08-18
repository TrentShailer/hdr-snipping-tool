use std::{fs::File, io::BufWriter};

use image::{codecs::png::PngEncoder, ImageBuffer, Rgba};

pub fn save_image(name: &'static str, data: Box<[u8]>, size: [u32; 2]) {
    let img: ImageBuffer<Rgba<u8>, Box<[u8]>> =
        ImageBuffer::from_raw(size[0], size[1], data).unwrap();

    let name = format!("{}.test.png", name);
    let file = File::create(name).unwrap();
    let mut buffer = BufWriter::new(file);
    let encoder = PngEncoder::new(&mut buffer);
    img.write_with_encoder(encoder).unwrap();
}
