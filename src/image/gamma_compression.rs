use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

/// a > 0; 0 < γ < 1;<br>
/// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
/// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
/// if a < 1 it can decrease the exposure of over exposed parts of the image.
pub fn compress_gamma(image: &[f32], alpha: f32, gamma: f32) -> Box<[u8]> {
    image
        .par_iter()
        .enumerate()
        .map(|(index, value)| {
            if (index + 1) % 4 == 0 {
                return (value.clone() * F32_255) as u8;
            }
            (compress_gamma_value(value, alpha, gamma) * F32_255) as u8
        })
        .collect()
}

fn compress_gamma_value(value: &f32, a: f32, gamma: f32) -> f32 {
    a * value.powf(gamma)
}

const F32_255: f32 = 255.0;
