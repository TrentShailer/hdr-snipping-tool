use std::time::Instant;

use half::f16;
use thiserror::Error;
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/maximum.spv"}
}

/// Performs a GPU reduction to find the largest value in the image
///
/// Staging buffer is a Transfer Source buffer that contains the f16 bytes to be reduced, it is unmodifed.
///
/// Bytecount is the number of bytes in the staging buffer
pub fn find_maximum(
    vk: &VulkanInstance,
    staging_buffer: Subbuffer<[u8]>,
    byte_count: u32,
) -> Result<f16, Error> {
    let start = Instant::now();
    let pipeline = {
        let shader = shader::load(vk.device.clone())
            .map_err(Error::LoadShader)?
            .entry_point("main")
            .unwrap();

        let stage = PipelineShaderStageCreateInfo::new(shader);

        let layout = PipelineLayout::new(
            vk.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                .into_pipeline_layout_create_info(vk.device.clone())
                .map_err(|e| Error::CreatePipelineLayoutInfo {
                    set_num: e.set_num,
                    error: e.error,
                })?,
        )
        .map_err(Error::CreatePipelineLayout)?;

        ComputePipeline::new(
            vk.device.clone(),
            None,
            ComputePipelineCreateInfo::stage_layout(stage, layout),
        )
        .map_err(Error::CreatePipeline)?
    };

    let subgroup_size = vk
        .device
        .physical_device()
        .properties()
        .subgroup_size
        .unwrap_or(1);

    // 1024 threads * two values per thread * sugroup_size
    // This is how much the input gets reduced by on a single pass
    let compute_blocksize = 1024 * 2 * subgroup_size;

    let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER
                | BufferUsage::TRANSFER_DST
                | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        byte_count as u64,
    )?;

    let output_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        (byte_count as u64).div_ceil(compute_blocksize as u64),
    )?;

    // Because one pass gets us input_length / compute_blocksize values
    // multiple passes may be required, therefore two descriptor sets are used
    // to swap input and output buffer around
    let layout = &pipeline.layout().set_layouts()[0];
    let descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, input_buffer.clone()),
            WriteDescriptorSet::buffer(1, output_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    let inverse_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, output_buffer.clone()),
            WriteDescriptorSet::buffer(1, input_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    let mut input_length = byte_count / 2;
    let mut output_length = (byte_count / 2).div_ceil(compute_blocksize);

    // copy data to input buffer
    copy_buffer_and_wait(
        vk,
        staging_buffer.clone(),
        input_buffer.clone(),
        copy_buffer::Region::SmallestBuffer,
    )?;

    // While there is multiple remaining candidates, do a pass
    // and swap the input and output buffer
    let mut use_inverse_set = false;
    while input_length > 1 {
        let workgroup_count = output_length;

        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        let set = if use_inverse_set {
            inverse_descriptor_set.clone()
        } else {
            descriptor_set.clone()
        };

        builder
            .bind_pipeline_compute(pipeline.clone())?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Compute,
                pipeline.layout().clone(),
                0,
                set,
            )?
            .push_constants(pipeline.layout().clone(), 0, input_length)?
            .dispatch([workgroup_count, 1, 1])?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        use_inverse_set = !use_inverse_set;
        input_length = output_length;
        output_length = input_length.div_ceil(compute_blocksize);
    }

    let result_buffer = if use_inverse_set {
        output_buffer.clone()
    } else {
        input_buffer.clone()
    };

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

    // Copy from result to staging
    copy_buffer_and_wait(
        vk,
        result_buffer,
        output_staging_buffer.clone(),
        copy_buffer::Region::SmallestBuffer,
    )?;

    let reader = &output_staging_buffer.read()?;
    let maximum = f16::from_le_bytes([reader[0], reader[1]]);

    let end = Instant::now();
    log::debug!(
        "Found maximum in {}ms",
        end.duration_since(start).as_millis()
    );

    Ok(maximum)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline layout info:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },

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
