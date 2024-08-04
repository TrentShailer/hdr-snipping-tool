use std::sync::Arc;

use half::f16;
use shader::PushConstants;
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage},
    descriptor_set::{layout::DescriptorSetLayout, PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo,
        layout::{IntoPipelineLayoutCreateInfoError, PipelineDescriptorSetLayoutCreateInfo},
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/maximum.spv"}
}

/// Performs a GPU reduction to find the largest value in the image.\
/// `input_buffer` is unmodifed.\
/// Bytecount is the number of bytes in the staging buffer.
pub(crate) fn find_maximum(
    vk: &VulkanInstance,
    input_buffer: Subbuffer<[u8]>,
    byte_count: u32,
) -> Result<f16, Error> {
    let _span = info_span!("find_maximum").entered();

    // Create pipline for maximum reduction
    let pipeline = {
        let shader = shader::load(vk.device.clone())
            .map_err(Error::LoadShader)?
            .entry_point("main")
            .unwrap();

        let stage = PipelineShaderStageCreateInfo::new(shader);

        let layout = PipelineLayout::new(
            vk.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(vk.device.clone())?,
        )
        .map_err(Error::CreatePipelineLayout)?;

        ComputePipeline::new(
            vk.device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .map_err(Error::CreatePipeline)?
    };

    // Query the subgroup size from the GPU
    let subgroup_size = vk.physical_device.properties().subgroup_size.unwrap_or(1);

    // 1024 threads * sugroup_size
    // This is how much the input gets reduced by on a single pass
    let compute_blocksize = 1024 * subgroup_size;

    // Setup "read" buffer
    let read_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        (byte_count as u64).div_ceil(compute_blocksize as u64) + 3,
    )?;

    // Setup "write" buffer
    let write_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        (byte_count as u64)
            .div_ceil(compute_blocksize as u64)
            .div_ceil(compute_blocksize as u64)
            + 3,
    )?;

    // Create descriptor sets
    let layout = &pipeline.layout().set_layouts()[0];
    let descriptor_sets = create_descriptor_sets(
        vk,
        layout,
        input_buffer.clone(),
        read_buffer.clone(),
        write_buffer.clone(),
    )?;

    let mut input_length = byte_count / 2;
    let mut output_length = (byte_count / 2).div_ceil(compute_blocksize);
    let mut ds_index = 0;

    while input_length > 1 {
        let _span = info_span!("pass").entered();
        let workgroup_count = output_length;

        // Perform reduction pass
        {
            let mut builder = AutoCommandBufferBuilder::primary(
                &vk.allocators.command,
                vk.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .map_err(Error::CreateCommandBuffer)?;

            builder
                .bind_pipeline_compute(pipeline.clone())?
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Compute,
                    pipeline.layout().clone(),
                    0,
                    descriptor_sets[ds_index].clone(),
                )?
                .push_constants(pipeline.layout().clone(), 0, PushConstants { input_length })?
                .dispatch([workgroup_count, 1, 1])?;

            // execute command buffer and wait for it to finish
            let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
            let future = sync::now(vk.device.clone())
                .then_execute(vk.queue.clone(), command_buffer)?
                .then_signal_fence_and_flush()
                .map_err(Error::SignalFence)?;
            future.wait(None).map_err(Error::AwaitFence)?;
        }

        // calculate updated input and output lengths
        input_length = output_length;
        output_length = input_length.div_ceil(compute_blocksize);

        // Swap the read and write buffers if there is a next run.
        if input_length > 1 {
            ds_index += 1;

            if ds_index > 2 {
                ds_index = 1;
            }
        }
    }

    // Find what buffer has the final result in it.
    let result_buffer = match ds_index {
        1 => write_buffer.clone(),
        2 => read_buffer.clone(),
        _ => input_buffer.clone(),
    };

    // Setup CPU staging buffer for GPU to write data to.
    let output_staging_buffer = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        4,
    )?;
    copy_buffer_and_wait(
        vk,
        result_buffer,
        output_staging_buffer.clone(),
        copy_buffer::Region::SmallestBuffer,
    )?;

    // Read the data from the buffer
    let reader = &output_staging_buffer.read()?;
    let maximum = f16::from_le_bytes([reader[0], reader[1]]);

    Ok(maximum)
}

fn create_descriptor_sets(
    vk: &VulkanInstance,
    layout: &Arc<DescriptorSetLayout>,
    input_buffer: Subbuffer<[u8]>,
    read_buffer: Subbuffer<[u8]>,
    write_buffer: Subbuffer<[u8]>,
) -> Result<[Arc<PersistentDescriptorSet>; 3], Error> {
    let first_pass_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, input_buffer.clone()),
            WriteDescriptorSet::buffer(1, read_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    let rw_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, read_buffer.clone()),
            WriteDescriptorSet::buffer(1, write_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    let wr_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, write_buffer.clone()),
            WriteDescriptorSet::buffer(1, read_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    Ok([
        first_pass_descriptor_set,
        rw_descriptor_set,
        wr_descriptor_set,
    ])
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline layout info:\n{0:?}")]
    CreatePipelineLayoutInfo(#[from] IntoPipelineLayoutCreateInfoError),

    #[error("Failed to create pipeline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    CreateDescriptorSet(#[source] Validated<VulkanError>),

    #[error("Failed to access buffer:\n{0}")]
    BufferAccess(#[from] HostAccessError),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to write to command buffer:\n{0}")]
    WriteCommandBuffer(#[from] Box<ValidationError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to signal fence:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] copy_buffer::Error),
}
