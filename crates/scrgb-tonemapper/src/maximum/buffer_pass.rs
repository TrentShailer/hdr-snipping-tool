use half::f16;
use shader::PushConstants;
use tracing::info_span;
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::{self, GpuFuture},
};

use super::Error;

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/maximum_buffer_pass.spv"}
}

/// Performs a gpu reduction over two buffers.
pub(crate) fn buffer_reduction(
    vk: &VulkanInstance,
    read_buffer: Subbuffer<[u8]>,
    write_buffer: Subbuffer<[u8]>,
    byte_count: u32,
) -> Result<f16, Error> {
    let _span = info_span!("buffer_reduction").entered();

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

    // Create descriptor sets
    let layout = &pipeline.layout().set_layouts()[0];
    let read_write_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, read_buffer.clone()),
            WriteDescriptorSet::buffer(1, write_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;
    let write_read_descriptor_set = PersistentDescriptorSet::new(
        &vk.allocators.descriptor,
        layout.clone(),
        [
            WriteDescriptorSet::buffer(0, write_buffer.clone()),
            WriteDescriptorSet::buffer(1, read_buffer.clone()),
        ],
        [],
    )
    .map_err(Error::CreateDescriptorSet)?;

    let mut input_length = byte_count / 2;
    let mut output_length = (byte_count / 2).div_ceil(compute_blocksize);
    let mut use_write_read_ds = true;

    while input_length > 1 {
        let _span = info_span!("pass").entered();
        let workgroup_count = output_length;

        use_write_read_ds = !use_write_read_ds;

        // Perform reduction pass
        {
            let mut builder = AutoCommandBufferBuilder::primary(
                &vk.allocators.command,
                vk.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .map_err(Error::CreateCommandBuffer)?;

            let descriptor_set = if use_write_read_ds {
                write_read_descriptor_set.clone()
            } else {
                read_write_descriptor_set.clone()
            };

            builder
                .bind_pipeline_compute(pipeline.clone())?
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Compute,
                    pipeline.layout().clone(),
                    0,
                    descriptor_set,
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
    }

    // Find what buffer has the final result in it.
    let result_buffer = if use_write_read_ds {
        read_buffer.clone()
    } else {
        write_buffer.clone()
    };

    // Setup CPU staging buffer for GPU to write data to.
    let output_staging_buffer = Buffer::new_slice(
        vk.allocators.memory.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_DST,
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
