use std::cmp::Ordering;

use image::{ImageBuffer, Rgba};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use super::{
    f32_from_f16_bytes::f32_from_le_f16_bytes, gamma_compression::compress_gamma, save_image,
};

pub struct Image {
    pub raw: Box<[f32]>,
    pub current: Box<[u8]>,
    pub width: u32,
    pub height: u32,
    pub alpha: f32,
    pub gamma: f32,
    pub selection_pos: [u32; 2],
    pub selection_size: [u32; 2],
}

impl Image {
    pub fn from_u8(slice: &[u8], width: u32, height: u32) -> Self {
        let raw = (0..slice.len() / 2)
            .into_par_iter()
            .map(|byte_index| {
                let start = byte_index * 2;
                f32_from_le_f16_bytes(slice[start], slice[start + 1])
            })
            .collect::<Box<[f32]>>();

        let max = Self::get_max_value(&raw);
        println!("{}", max);

        let gamma = 0.5;
        let alpha = max.powf(-gamma);
        let current = compress_gamma(&raw, alpha, gamma);

        Self {
            raw,
            current,
            width,
            height,
            alpha,
            gamma,
            selection_pos: [0, 0],
            selection_size: [width, height],
        }
    }

    pub fn blank() -> Self {
        Self {
            raw: Box::new([]),
            current: Box::new([]),
            width: 0,
            height: 0,
            alpha: 0.0,
            gamma: 0.0,
            selection_pos: [0, 0],
            selection_size: [0, 0],
        }
    }

    pub fn compress_gamma(&mut self) {
        self.current = compress_gamma(&self.raw, self.alpha, self.gamma);
    }

    pub fn get_selection_rect(&self) -> [[f32; 2]; 2] {
        let pos = self.selection_pos;
        let size = self.selection_size;

        [
            [pos[0] as f32, pos[1] as f32],
            [
                pos[0] as f32 + size[0] as f32,
                pos[1] as f32 + size[1] as f32,
            ],
        ]
    }

    pub fn calculate_alpha(&self) -> f32 {
        let max = Self::get_max_value(&self.raw);
        max.powf(-self.gamma)
    }

    pub fn get_max_value(image: &[f32]) -> &f32 {
        image
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

                a.total_cmp(b)
            })
            .unwrap()
            .1
    }

    pub fn save(&self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        save_image(
            &self.current,
            self.selection_pos,
            self.selection_size,
            self.width,
            self.height,
        )
    }
}
