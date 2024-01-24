use chrono::Local;
use image::{ColorType, ImageFormat, ImageResult};

pub fn save_jpeg(image: &[u8], width: u32, height: u32) -> ImageResult<()> {
    let name = format!("screenshot {}.jpg", Local::now().format("%F %l-%M-%S %P"));

    image::save_buffer_with_format(
        name,
        image,
        width,
        height,
        ColorType::Rgba8,
        ImageFormat::Jpeg,
    )?;

    Ok(())
}
