use half::f16;

pub fn convert_image_to_bytes(
    image: Vec<Vec<[f16; 4]>>,
    width: usize,
    height: usize,
    image_max: f16,
) -> Vec<u8> {
    let mut bytes = vec![0u8; width * height * 4];

    for row in 0..height {
        for pixel in 0..width {
            for channel in 0..4 {
                let byte_index = channel + (pixel * 4) + (row * width * 4);

                let channel_value = image[row][pixel][channel];
                if channel == 3 {
                    bytes[byte_index] = convert_channel(channel_value, f16::ONE);
                } else {
                    bytes[byte_index] = convert_channel(channel_value, image_max);
                }
            }
        }
    }

    bytes
}

fn convert_channel(channel: f16, image_max: f16) -> u8 {
    // scape channel between 0 and 1
    let mut value = channel / image_max;
    value *= f16::from_f32(255.0);

    let value_f32 = value.to_f32();
    value_f32 as u8
}
