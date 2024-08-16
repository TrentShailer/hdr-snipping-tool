pub mod maximum;
pub mod tonemap_output;

use std::{fmt::Debug, sync::Arc};

use ash::vk::ImageView;
use thiserror::Error;
use tonemap_output::TonemapOutput;
use tracing::info_span;
use vulkan_instance::VulkanInstance;

/// Tonemaps a capture from the scRGB colorspace into the sRGB colorspace.\
/// Returns a vulkan image containing the capture.
pub fn tonemap(
    vk: &VulkanInstance,
    capture: Arc<ImageView>,
    capture_size: [u32; 2],
    whitepoint: f32,
) -> Result<TonemapOutput, Error> {
    let _span = info_span!("tonemap").entered();

    // Create output image
    let capture_output = TonemapOutput::new(vk, capture_size)?;

    // Setup compute pipline
    let pipeline = {
        let compute_shader = shader::load(vk.device.clone())
            .map_err(Error::LoadShader)?
            .specialize([(0, whitepoint.into())].into_iter().collect())
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

    // Setup descriptor set
    let io_layout = &pipeline.layout().set_layouts()[0];
    let io_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        io_layout.clone(),
        [
            WriteDescriptorSet::image_view(0, capture.clone()),
            WriteDescriptorSet::image_view(1, capture_output.image_view.clone()),
        ],
        [],
    )
    .map_err(Error::Descriptor)?;

    // Dispatch tonemapper
    let dispatch_span = info_span!("dispatch").entered();

    // Shader tonemaps a 32x32 area each dispatch
    let workgroup_x = capture_size[0].div_ceil(32);
    let workgroup_y = capture_size[1].div_ceil(32);

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
        .dispatch([workgroup_x, workgroup_y, 1])?;

    // Build and execute the command buffer, then wait for it to finish.
    let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
    let future = sync::now(vk.device.clone())
        .then_execute(vk.queue.clone(), command_buffer)?
        .then_signal_fence_and_flush()
        .map_err(Error::SignalFenceAndFlush)?;
    future.wait(None).map_err(Error::AwaitFence)?;
    dispatch_span.exit();

    Ok(capture_output)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,
}
