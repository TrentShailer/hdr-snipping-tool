use std::sync::Arc;

use tracing::info_span;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::view::ImageView,
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::{self, GpuFuture},
};

use super::Error;

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/maximum_source_pass.spv"}
}

/// Performs a gpu reduction pass over an image.
pub(crate) fn source_reduction_pass(
    vk: &VulkanInstance,
    source: Arc<ImageView>,
    source_size: [u32; 2],
    output_buffer: Subbuffer<[u8]>,
) -> Result<(), Error> {
    let _span = info_span!("source_reduction_pass").entered();

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

    // Create descriptor set
    let layout = &pipeline.layout().set_layouts()[0];
    let descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::image_view(0, source.clone()),
            WriteDescriptorSet::buffer(1, output_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    // Query the subgroup size from the GPU
    let subgroup_size = vk.physical_device.properties().subgroup_size.unwrap_or(1);

    let workgroup_x = source_size[0].div_ceil(32);
    let workgroup_y = source_size[1].div_ceil(32).div_ceil(subgroup_size);

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
                descriptor_set,
            )?
            .dispatch([workgroup_x, workgroup_y, 1])?;

        // execute command buffer and wait for it to finish
        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;
    }

    Ok(())
}
