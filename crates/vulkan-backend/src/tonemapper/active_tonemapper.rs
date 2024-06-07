use std::sync::Arc;

use half::f16;
use vulkano::{buffer::Subbuffer, descriptor_set::PersistentDescriptorSet};
use winit::dpi::PhysicalSize;

use super::shader;

pub struct ActiveTonemapper {
    pub capture_size: PhysicalSize<u32>,
    pub input_size: u32,
    pub alpha: f16,
    pub gamma: f16,
    pub maximum: f16,
    pub input_buffer: Subbuffer<[u8]>,
    pub output_buffer: Subbuffer<[u8]>,
    pub config_buffer: Subbuffer<shader::Config>,
    pub descriptor_set_0: Arc<PersistentDescriptorSet>,
    pub descriptor_set_1: Arc<PersistentDescriptorSet>,
}
