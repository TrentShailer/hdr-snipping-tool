use image::{ColorType, ImageFormat, ImageResult};

pub fn write_image(
    image: &[u8],
    width: u32,
    height: u32,
    name: &str,
    color_type: ColorType,
    format: ImageFormat,
) -> ImageResult<()> {
    image::save_buffer_with_format(name, image, width, height, color_type, format)?;

    Ok(())
}
