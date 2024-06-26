use std::sync::Arc;

use tonemapper::shader::Config;
use vulkano::{
    buffer::Subbuffer, descriptor_set::PersistentDescriptorSet, pipeline::ComputePipeline,
    query::QueryPool,
};

pub mod find_maximum;
pub mod tonemapper;

pub struct Tonemapper {
    pub pipeline: Arc<ComputePipeline>,
    pub config: Config,
    pub input_buffer: Subbuffer<[u8]>,
    pub config_buffer: Subbuffer<Config>,
    pub io_set: Arc<PersistentDescriptorSet>,
    pub config_set: Arc<PersistentDescriptorSet>,
    pub timestamp_pool: Option<Arc<QueryPool>>,
}
