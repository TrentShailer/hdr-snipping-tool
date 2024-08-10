pub mod load;
pub mod render;

use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::Subbuffer,
    descriptor_set::PersistentDescriptorSet,
    image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
    pipeline::GraphicsPipeline,
    Validated, VulkanError,
};

use crate::{pipelines::capture::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

pub struct Capture {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub sampler: Arc<Sampler>,
    pub capture_ds: Option<Arc<PersistentDescriptorSet>>,
    pub whitepoint: f32,
}

impl Capture {
    pub fn new(vk: &VulkanInstance, pipeline: Arc<GraphicsPipeline>) -> Result<Self, Error> {
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 0.0],
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                uv: [1.0, 0.0],
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, 1.0],
            }, // BL
        ];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        let sampler = Sampler::new(
            vk.device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::Repeat; 3],
                ..Default::default()
            },
        )
        .map_err(Error::Sampler)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            sampler,
            capture_ds: None,
            whitepoint: 0.0,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create vertex and index buffer:\n{0}")]
    CreateVertexIndexBuffer(#[from] crate::vertex_index_buffer::Error),

    #[error("Failed to create sampler:\n{0:?}")]
    Sampler(#[source] Validated<VulkanError>),
}
