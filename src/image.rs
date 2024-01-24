use std::{cmp::Ordering, time::SystemTime};

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};

pub struct Image {
    pub values: Box<[f32]>,
    pub width: usize,
    pub height: usize,
}

impl Image {
    pub fn from_u8(slice: &[u8], width: usize, height: usize) -> Self {
        let values = (0..slice.len() / 2)
            .into_par_iter()
            .map(|byte_index| {
                let start = byte_index * 2;
                Self::f32_from_le_f16_bytes(slice[start], slice[start + 1])
            })
            .collect::<Box<[f32]>>();

        Self {
            values,
            width,
            height,
        }
    }

    fn f32_from_le_f16_bytes(byte_0: u8, byte_1: u8) -> f32 {
        let sign: u8 = byte_1 & 0b1000_0000;
        let exponent: u8 = ((byte_1 & 0b0111_1100) >> 2) + 0b01110000;
        let fration_l: u8 = (byte_1 & 0b0000_0011) << 5 | byte_0 >> 3;
        let fration_r: u8 = byte_0 << 5;

        f32::from_le_bytes([
            0,
            fration_r,
            exponent << 7 | fration_l,
            sign | exponent >> 1,
        ])
    }

    /// a > 0; 0 < γ < 1;<br>
    /// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
    /// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
    /// if a < 1 it can decrease the exposure of over exposed parts of the image.
    pub fn compress_gamma(&mut self, a: f32, gamma: f32) {
        self.values
            .par_iter_mut()
            .enumerate()
            .for_each(|(index, value)| {
                if (index + 1) % 4 == 0 {
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

    pub fn as_bytes(&self) -> Box<[u8]> {
        self.values
            .par_iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    pub fn into_bytes(self) -> Box<[u8]> {
        let max_start = SystemTime::now();
        let (_max_value_index, max_value) = self.get_max_value();
        let max_end = SystemTime::now();
        let duration = max_end.duration_since(max_start).unwrap();
        println!("Max took {}s", duration.as_secs_f64());

        let u8_start = SystemTime::now();
        let bytes = self
            .values
            .into_par_iter()
            .enumerate()
            .map(|(index, value)| {
                if (index + 1) % 4 == 0 {
                    (value * F32_255) as u8
                } else {
                    (value / max_value * F32_255) as u8
                }
            })
            .collect::<Box<[u8]>>();

        let u8_end = SystemTime::now();
        let duration = u8_end.duration_since(u8_start).unwrap();
        println!("u8 took {}s", duration.as_secs_f64());

        bytes
    }
}

const F32_255: f32 = 255.0;
