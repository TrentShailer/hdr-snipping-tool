use std::{cmp::Ordering, time::SystemTime};

use half::f16;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

pub struct Image {
    pub values: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl Image {
    pub fn from_u8(slice: &[u8], width: usize, height: usize) -> Self {
        let values = (0..slice.len() / 2)
            .into_par_iter()
            .map(|byte_index| {
                let mut channel = [0u8; 2];
                let start = byte_index * 2;
                let end = start + 2;
                channel[..2].copy_from_slice(&slice[start..end]);
                f16::from_le_bytes(channel).to_f32()
            })
            .collect::<Vec<f32>>();

        Self {
            values: values,
            width,
            height,
        }
    }

    // fn f32_from_f16_bytes(bytes: [u8; 2]) -> f32 {}

    /// a > 0; 0 < γ < 1;<br>
    /// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
    /// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
    /// if a < 1 it can decrease the exposure of over exposed parts of the image.
    pub fn compress_gamma(&mut self, a: f32, gamma: f32) {
        self.values
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, value)| {
                if index + 1 % 4 == 0 {
                    return;
                }
                Self::compress_gamma_value(value, a, gamma);
            });
    }

    fn compress_gamma_value(value: &mut f32, a: f32, gamma: f32) {
        *value = a * value.powf(gamma);
    }

    fn get_max_value(&self) -> (usize, &f32) {
        self.values
            .par_iter()
            .enumerate()
            .max_by(|(a_index, a), (b_index, b)| {
                let is_a_alpha = (a_index + 1) % 4 == 0;
                let is_b_alpha = (b_index + 1) % 4 == 0;

                if is_a_alpha && !is_b_alpha {
                    return Ordering::Less;
                }

                if !is_a_alpha && is_b_alpha {
                    return Ordering::Greater;
                }

                if is_a_alpha && is_b_alpha {
                    return Ordering::Equal;
                }

                a.partial_cmp(b).unwrap()
            })
            .unwrap()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let max_start = SystemTime::now();
        let (max_value_index, max_value) = self.get_max_value();
        let max_end = SystemTime::now();
        let duration = max_end.duration_since(max_start).unwrap();
        println!("Max took {}s", duration.as_secs_f64());

        let u8_start = SystemTime::now();
        let bytes = self
            .values
            .to_owned()
            .into_par_iter()
            .enumerate()
            .map(|(index, value)| {
                if (index + 1) % 4 == 0 {
                    (value * 255.0) as u8
                } else {
                    Self::scale_0_1(value, &max_value) as u8
                }
            })
            .collect::<Vec<u8>>();

        let u8_end = SystemTime::now();
        let duration = u8_end.duration_since(u8_start).unwrap();
        println!("u8 took {}s", duration.as_secs_f64());

        bytes
    }

    fn scale_0_1(input: f32, max_value: &f32) -> f32 {
        input / max_value * 255.0
    }
}
