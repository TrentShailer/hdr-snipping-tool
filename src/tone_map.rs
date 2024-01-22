use half::f16;

/// a > 0; 0 < γ < 1;<br>
/// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
/// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
/// if a < 1 it can decrease the exposure of over exposed parts of the image.
pub fn gamma_compression(image: &mut Vec<Vec<[f16; 4]>>, a: f32, γ: f32) {
    for row in image.iter_mut() {
        for pixel in row.iter_mut() {
            for channel in 0..4 {
                if channel != 3 {
                    perform_gamma_compression(&mut pixel[channel], a, γ);
                }
            }
        }
    }
}

fn perform_gamma_compression(channel: &mut f16, a: f32, γ: f32) {
    let f32_value = f16::to_f32(channel.to_owned());
    let new_value = a * f32_value.powf(γ);
    *channel = f16::from_f32(new_value);
}
