use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

use half::f16;
use rayon::iter::{
    IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

pub struct Image {
    pub rows: Vec<Vec<[f16; 4]>>,
    pub width: usize,
    pub height: usize,
}

impl Image {
    pub fn from(slice: &[u8], row_pitch: usize, width: usize, height: usize) -> Self {
        let rows = (0..height)
            .into_par_iter()
            .map(|row| {
                let slice_begin = (row * row_pitch) as usize;
                let slice_end = slice_begin + (width * 8);

                let slice = &slice[slice_begin..slice_end];

                (0..(slice.len() / 8))
                    .into_par_iter()
                    .map(|pixel_index| {
                        let mut pixel = [f16::ZERO; 4];

                        for channel_index in 0..4 {
                            let channel_start = (pixel_index * 8) + (channel_index * 2);
                            let mut channel = [0u8; 2];

                            for byte_index in 0..2 {
                                channel[byte_index] = slice[channel_start + byte_index];
                            }

                            let pixel_value = f16::from_le_bytes(channel);
                            pixel[channel_index] = pixel_value;
                        }

                        pixel
                    })
                    .collect::<Vec<[f16; 4]>>()
            })
            .collect::<Vec<Vec<[f16; 4]>>>();

        Self {
            rows,
            width,
            height,
        }
    }

    pub fn new(width: usize, height: usize) -> Self {
        let rows = vec![vec![[f16::ZERO; 4]; width]; height];
        Self {
            rows,
            width,
            height,
        }
    }

    /// a > 0; 0 < γ < 1;<br>
    /// Maps from the domain \[0,a^(-1/γ)] to the domain \[0,1].<br>
    /// γ regulated contrast, lower = lower, but also increases exposure of underexposed parts if lower.<br>
    /// if a < 1 it can decrease the exposure of over exposed parts of the image.
    pub fn compress_gamma(&mut self, a: f32, γ: f32) {
        // let mut max_value = f16::ZERO;
        self.rows.par_iter_mut().for_each(|row| {
            row.par_iter_mut().for_each(|pixel| {
                for channel in 0..3 {
                    Self::compress_gamma_value(&mut pixel[channel], a, γ);
                }
            })
        });
    }

    fn compress_gamma_value(channel: &mut f16, a: f32, γ: f32) {
        let f32_value = f16::to_f32(channel.to_owned());
        let new_value = a * f32_value.powf(γ);
        *channel = f16::from_f32(new_value);
    }

    fn get_max_value(&self) -> &f16 {
        self.rows
            .par_iter()
            .flatten_iter()
            .flatten_iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let max_start = SystemTime::now();
        let max_value = self.get_max_value().to_f32();
        let max_end = SystemTime::now();
        let duration = max_end.duration_since(max_start).unwrap();
        println!("Max took {}s", duration.as_secs_f64());

        let u8_start = SystemTime::now();
        let bytes = (0..self.width * self.height * 4)
            .into_par_iter()
            .map(|index| {
                let row = index / (self.width * 4);
                let pixel = (index / 4) % self.width;
                let channel = index % 4;

                let value = self.rows[row][pixel][channel].to_f32();

                if channel == 3 {
                    value as u8
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
