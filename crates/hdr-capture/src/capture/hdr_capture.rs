use std::cmp::Ordering;

use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use winit::dpi::PhysicalSize;

/// An HDR capture using using the RGBA_f32 pixel format
#[derive(Default, Debug)]
pub struct HdrCapture {
    pub data: Box<[f32]>,
    pub size: PhysicalSize<u32>,
}

impl HdrCapture {
    pub fn new(data: Box<[f32]>, size: PhysicalSize<u32>) -> Self {
        Self { data, size }
    }

    pub fn get_max_value(&self) -> &f32 {
        self.data
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
}
