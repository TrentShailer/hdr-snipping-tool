pub mod render;
pub mod set_text;
pub mod update_glyph_cache;

use std::sync::Arc;

use fontdue::layout::Layout;
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline},
    Validated, VulkanError,
};

use crate::{
    glyph_cache::GlyphCache,
    pipelines::text::{InstanceData, Vertex},
    vertex_index_buffer::{self, create_vertex_and_index_buffer},
};

pub struct Text {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub instance_buffer: Subbuffer<[InstanceData]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub instances: u32,
    pub text: String,
    //
    pub layout: Layout,
    pub atlas_ds: Arc<PersistentDescriptorSet>,
    pub extent: [f32; 2],
}

impl Text {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        glyph_cache: &GlyphCache,
        instance_capacity: u32,
        color: [u8; 4],
    ) -> Result<Self, Error> {
        let layout = Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);

        // verticies
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 0.0],
                color,
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                uv: [1.0, 0.0],
                color,
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
                color,
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, 1.0],
                color,
            }, // BL
        ];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, verticies, indicies)?;

        let instance_buffer: Subbuffer<[InstanceData]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            instance_capacity as u64,
        )?;

        let atlas_ds_layout = pipeline.layout().set_layouts()[0].clone();

        let atlas_ds = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            atlas_ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, glyph_cache.atlas_sampler.clone()),
                WriteDescriptorSet::image_view(1, glyph_cache.atlas_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            pipeline,
            instances: 0,
            text: String::new(),
            //
            layout,
            atlas_ds,
            extent: [0.0, 0.0],
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create vertex and index buffer:\n{0}")]
    VertexIndex(#[from] vertex_index_buffer::Error),

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
