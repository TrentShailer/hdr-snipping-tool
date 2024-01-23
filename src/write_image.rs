use image::ImageResult;

pub fn write_image(image: Vec<u8>, width: u32, height: u32) -> ImageResult<()> {
    image::save_buffer_with_format(
        "screenshot.png",
        &image,
        width,
        height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )?;

    Ok(())
}
