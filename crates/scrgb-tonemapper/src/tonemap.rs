use std::{sync::Arc, time::Instant};

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage},
    descriptor_set::PersistentDescriptorSet,
    pipeline::{ComputePipeline, Pipeline},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

use crate::shader::Metadata;

/// Dispatches the tonemapper.
pub(crate) fn dispatch_tonemap(
    vk: &VulkanInstance,
    size: [u32; 2],
    pipeline: Arc<ComputePipeline>,
    io_set: Arc<PersistentDescriptorSet>,
    sdr_whitepoint: f32,
    hdr_whitepoint: f32,
    maximum: f32,
) -> Result<(), Error> {
    let start = Instant::now();

    // Shader tonemaps a 32x32 area each dispatch
    let workgroup_x = size[0].div_ceil(32);
    let workgroup_y = size[1].div_ceil(32);

    // Create command buffer for dispatch
    let mut builder = AutoCommandBufferBuilder::primary(
        &vk.allocators.command,
        vk.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .map_err(Error::CreateCommandBuffer)?;

    // Bind pipline, ds, push constants
    // then dispatch enough workers in the x and y axis to cover the capture
    builder
        .bind_pipeline_compute(pipeline.clone())?
        .bind_descriptor_sets(
            vulkano::pipeline::PipelineBindPoint::Compute,
            pipeline.layout().clone(),
            0,
            io_set.clone(),
        )?
        .push_constants(
            pipeline.layout().clone(),
            0,
            Metadata {
                sdr_whitepoint,
                hdr_whitepoint,
                maximum,
            },
        )?
        .dispatch([workgroup_x, workgroup_y, 1])?;

    // Build and execute the command buffer, then wait for it to finish.
    let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
    let future = sync::now(vk.device.clone())
        .then_execute(vk.queue.clone(), command_buffer)?
        .then_signal_fence_and_flush()
        .map_err(Error::SignalFenceAndFlush)?;
    future.wait(None).map_err(Error::AwaitFence)?;

    log::debug!(
        "[dispatch_tonemap]
  [CPU TIMING] {}ms",
        start.elapsed().as_millis(),
    );

    Ok(())
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to write to command buffer:\n{0}")]
    WriteCommandBuffer(#[from] Box<ValidationError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to signal fence and flush:\n{0:?}")]
    SignalFenceAndFlush(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),
}
