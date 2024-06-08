pub mod reduce;

use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    Validated, VulkanError,
};

use crate::allocators::Allocators;

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "shaders/compute/maximum_reduction.spv"}
}

/// Because allocating a buffer takes up processing time
/// pre-allocating it during pipline creation and reusing it
/// is more efficent.<br>
/// 2^26 bytes = 67,108,864 bytes = ~67Mb <br>
/// 8 bytes per pixel = at most 8,388,608 pixels
pub const MAXIMUM_INPUT_BUFFER_SIZE: u64 = u64::pow(2, 26);

pub struct MaximumReducer {
    inverse_descriptor_set: Arc<PersistentDescriptorSet>,
    descriptor_set: Arc<PersistentDescriptorSet>,
    pipeline: Arc<ComputePipeline>,
    compute_blocksize: u32,
    output_buffer: Subbuffer<[u8]>,
    input_buffer: Subbuffer<[u8]>,
}

impl MaximumReducer {
    pub fn new(device: Arc<Device>, allocators: Arc<Allocators>) -> Result<Self, Error> {
        let subgroup_size = device
            .physical_device()
            .properties()
            .subgroup_size
            .unwrap_or(1);

        // 1024 threads * two values per thread * sugroup_size
        let compute_blocksize = 1024 * 2 * subgroup_size;

        let shader = shader::load(device.clone())
            .map_err(Error::LoadShader)?
            .entry_point("main")
            .unwrap(); // Unwrap is safe because of known entrypoint;

        let pipeline = {
            let stage = PipelineShaderStageCreateInfo::new(shader);

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(device.clone())
                    .map_err(|e| Error::CreatePipelineLayoutInfo {
                        set_num: e.set_num,
                        error: e.error,
                    })?,
            )
            .map_err(Error::CreatePipelineLayout)?;

            ComputePipeline::new(
                device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .map_err(Error::CreatePipeline)?
        };

        let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            MAXIMUM_INPUT_BUFFER_SIZE,
        )?;

        let output_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            MAXIMUM_INPUT_BUFFER_SIZE.div_ceil(compute_blocksize as u64),
        )?;

        // Because one pass gets us input_length / compute_blocksize values
        // multiple passes may be required, therefore two descriptor sets are used
        // to swap input and output buffer around
        let layout = &pipeline.layout().set_layouts()[0];
        let descriptor_set = PersistentDescriptorSet::new(
            &allocators.descriptor,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, input_buffer.clone()),
                WriteDescriptorSet::buffer(1, output_buffer.clone()),
            ],
            [],
        )
        .map_err(Error::CreateDescriptorSet)?;

        let inverse_descriptor_set = PersistentDescriptorSet::new(
            &allocators.descriptor,
            layout.clone(),
            [
                WriteDescriptorSet::buffer(0, output_buffer.clone()),
                WriteDescriptorSet::buffer(1, input_buffer.clone()),
            ],
            [],
        )
        .map_err(Error::CreateDescriptorSet)?;

        Ok(Self {
            inverse_descriptor_set,
            descriptor_set,
            pipeline,
            compute_blocksize,
            output_buffer,
            input_buffer,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Load Sahder Error:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Create Pipeline Layout Info Error:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },

    #[error("Create Pipeline Layout Error:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Create Pipeline Error:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Create Buffer Error:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Create Descriptor Set Error:\n{0:?}")]
    CreateDescriptorSet(#[source] Validated<VulkanError>),
}
