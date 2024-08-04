pub mod find_maximum;
pub mod tonemap;
pub mod tonemap_output;

use std::fmt::Debug;

use half::f16;
use thiserror::Error;
use tonemap::dispatch_tonemap;
use tonemap_output::TonemapOutput;
use tracing::{info, info_span};
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};

use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo,
        layout::{IntoPipelineLayoutCreateInfoError, PipelineDescriptorSetLayoutCreateInfo},
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::HostAccessError,
    Validated, ValidationError, VulkanError,
};
use windows_capture_provider::capture::Capture;

use crate::find_maximum::find_maximum;

mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/scRGB_to_sRGB.spv"}
}

/// Tonemaps a capture from the scRGB colorspace into the sRGB colorspace.\
/// Returns a vulkan image containing the capture.
pub fn tonemap(
    vk: &VulkanInstance,
    capture: &Capture,
    hdr_whitepoint: f32,
) -> Result<TonemapOutput, Error> {
    let _span = info_span!("tonemap").entered();

    // Transfer capture to staging buffer
    let staging_span = info_span!("write_staging").entered();
    let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_HOST
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        capture.data.len() as u64,
    )?;
    staging_buffer.write()?.copy_from_slice(&capture.data);
    staging_span.exit();

    // Transfer staging buffer to GPU
    let input_span = info_span!("write_input").entered();
    let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        capture.data.len() as u64,
    )?;
    copy_buffer_and_wait(
        vk,
        staging_buffer.clone(),
        input_buffer.clone(),
        vulkan_instance::copy_buffer::Region::SmallestBuffer,
    )?;
    input_span.exit();

    // find the brightest component in the capture.
    let max = find_maximum(vk, input_buffer.clone(), capture.data.len() as u32)?;
    // Sometimes the maximum is increased by an f16 step over the sdr_reference_white
    // Check for this case and use the sdr reference white if it is
    let max = if f16::from_bits(max.to_bits() - 1).to_f32() == capture.display.sdr_referece_white {
        capture.display.sdr_referece_white
    } else {
        max.to_f32()
    };

    // Create output image
    let capture_output = TonemapOutput::new(vk, capture.display.size)?;

    // Setup compute pipline
    let pipeline = {
        let compute_shader = shader::load(vk.device.clone())
            .map_err(Error::LoadShader)?
            .specialize(
                [
                    (0, capture.display.sdr_referece_white.into()),
                    (1, hdr_whitepoint.into()),
                    (2, max.into()),
                ]
                .into_iter()
                .collect(),
            )
            .map_err(Error::Specialize)?
            .entry_point("main")
            .unwrap();

        let stage = PipelineShaderStageCreateInfo::new(compute_shader);

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

    // Setip descriptor set
    let io_layout = &pipeline.layout().set_layouts()[0];
    let io_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        io_layout.clone(),
        [
            WriteDescriptorSet::buffer(0, input_buffer.clone()),
            WriteDescriptorSet::image_view(1, capture_output.image_view.clone()),
        ],
        [],
    )
    .map_err(Error::Descriptor)?;

    info!(
        sdr_white = capture.display.sdr_referece_white,
        hdr_white = hdr_whitepoint,
        maximum = max
    );

    // Dispatch tonemapper
    dispatch_tonemap(vk, capture.display.size, pipeline, io_set)?;

    Ok(capture_output)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to allocate buffer:\n{0:?}")]
    AllocateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0:?}")]
    BufferAccess(#[from] HostAccessError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] copy_buffer::Error),

    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to specialize shader:\n{0:?}")]
    Specialize(#[source] Box<ValidationError>),

    #[error("Failed to create pipeline layout info:\n{0:?}")]
    CreatePipelineLayoutInfo(#[from] IntoPipelineLayoutCreateInfoError),

    #[error("Failed to create pipeline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    Descriptor(#[source] Validated<VulkanError>),

    #[error("Failed to find capture maximum:\n{0}")]
    Maximum(#[from] find_maximum::Error),

    #[error("Failed to create tonemap output image:\n{0}")]
    TonemapOutput(#[from] tonemap_output::Error),

    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] tonemap::Error),
}
