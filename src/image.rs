use std::{
    borrow::{Borrow, Cow},
    cmp::Ordering,
    sync::Arc,
};

use egui::{ColorImage, ImageSource};
use image::{DynamicImage, RgbaImage};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use crate::write_image::save_jpeg;

pub struct Image {
    pub raw: Box<[f32]>,
    pub current: Box<[u8]>,
    pub width: usize,
    pub height: usize,
    pub alpha: f32,
    pub gamma: f32,
}

impl Image {
    pub fn from_u8(slice: &[u8], width: usize, height: usize) -> Self {
        let raw = (0..slice.len() / 2)
            .into_par_iter()
            .map(|byte_index| {
                let start = byte_index * 2;
                Self::f32_from_le_f16_bytes(slice[start], slice[start + 1])
            })
            .collect::<Box<[f32]>>();

        let max = Self::get_max_value(&raw).1;
        let gamma = 0.5;
        let alpha = max.powf(-gamma);
        let current = Self::compress_gamma(&raw, alpha, gamma);

        Self {
            raw,
            current,
            width,
            height,
            alpha,
            gamma,
        }
    }

    pub fn empty() -> Self {
        Self {
            raw: Box::new([]),
            current: Box::new([]),
            width: 0,
            height: 0,
            alpha: 0.0,
            gamma: 0.0,
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

    pub fn calculate_alpha(&self) -> f32 {
        let max = Self::get_max_value(&self.raw).1;
        max.powf(-self.gamma)
    }

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
                (Self::compress_gamma_value(value, alpha, gamma) * F32_255) as u8
            })
            .collect()
    }

    fn compress_gamma_value(value: &f32, a: f32, gamma: f32) -> f32 {
        a * value.powf(gamma)
    }

    pub fn get_max_value(image: &[f32]) -> (usize, &f32) {
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
    }

    pub fn as_bytes(&self) -> Box<[u8]> {
        self.raw
            .par_iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    pub fn get_color_image(&self) -> ColorImage {
        egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.current)
    }

    /* pub fn as_rgba8(&self) -> Box<[u8]> {
        self.current
            .par_iter()
            .map(|value| (value * F32_255) as u8)
            .collect::<Box<[u8]>>()
    } */

    pub fn save(&self) {
        save_jpeg(&self.current, self.width as u32, self.height as u32).unwrap();
    }
}

const F32_255: f32 = 255.0;

impl<'a> Into<ImageSource<'a>> for Image {
    fn into(self) -> ImageSource<'a> {
        ImageSource::Bytes {
            uri: std::borrow::Cow::Borrowed(""),
            bytes: egui::load::Bytes::Shared(self.current.into()),
        }
    }
}
