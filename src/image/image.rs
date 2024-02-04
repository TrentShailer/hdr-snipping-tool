use std::cmp::Ordering;

use glium::glutin::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use image::{ImageBuffer, Rgba};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use super::{
    f32_from_f16_bytes::f32_from_le_f16_bytes, gamma_compression::compress_gamma, save_image,
};

#[derive(Clone)]
pub struct Image {
    pub raw: Box<[f32]>,
    pub current: Box<[u8]>,
    pub size: PhysicalSize<u32>,
    pub alpha: f32,
    pub gamma: f32,
    pub selection_pos: PhysicalPosition<u32>,
    pub selection_size: PhysicalSize<u32>,
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

        let gamma = 0.5;
        let alpha = max.powf(-gamma);
        let current = compress_gamma(&raw, alpha, gamma);

        Self {
            raw,
            current,
            size: PhysicalSize::new(width, height),
            alpha,
            gamma,
            selection_pos: PhysicalPosition::new(0, 0),
            selection_size: PhysicalSize::new(width, height),
        }
    }

    pub fn blank() -> Self {
        Self {
            raw: Box::new([]),
            current: Box::new([]),
            size: PhysicalSize::new(0, 0),
            alpha: 0.0,
            gamma: 0.0,
            selection_pos: PhysicalPosition::new(0, 0),
            selection_size: PhysicalSize::new(0, 0),
        }
    }

    pub fn compress_gamma(&mut self) {
        self.current = compress_gamma(&self.raw, self.alpha, self.gamma);
    }

    pub fn get_selection_rect(&self, scale_factor: f64) -> [LogicalPosition<f32>; 2] {
        let pos = self.selection_pos.to_logical(scale_factor);
        let size: LogicalSize<f32> = self.selection_size.to_logical(scale_factor);

        [
            pos,
            LogicalPosition::new(pos.x + size.width, pos.y + size.height),
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
            self.size,
        )
    }
}
