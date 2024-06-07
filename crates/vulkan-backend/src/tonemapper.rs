pub mod active_tonemapper;
pub mod load_capture;
pub mod maximum_reducer;
pub mod tonemap;
pub mod utils;

use std::sync::Arc;

use active_tonemapper::ActiveTonemapper;
use maximum_reducer::MaximumReducer;
use thiserror::Error;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, Queue},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    Validated, VulkanError,
};

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/tonemap.spv"}
}

pub struct Tonemapper {
    maximum_reducer: MaximumReducer,
    active_tonemapper: Option<ActiveTonemapper>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    mem_alloc: Arc<StandardMemoryAllocator>,
    ds_alloc: Arc<StandardDescriptorSetAllocator>,
    cb_alloc: Arc<StandardCommandBufferAllocator>,
    pipeline: Arc<ComputePipeline>,
}

impl Tonemapper {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        mem_alloc: Arc<StandardMemoryAllocator>,
        ds_alloc: Arc<StandardDescriptorSetAllocator>,
        cb_alloc: Arc<StandardCommandBufferAllocator>,
    ) -> Result<Self, Error> {
        let maximum_reducer = MaximumReducer::new(
            device.clone(),
            queue.clone(),
            mem_alloc.clone(),
            ds_alloc.clone(),
            cb_alloc.clone(),
        )?;

        let pipeline = {
            let compute_shader = shader::load(device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap(); // Only none if 'main' doesn't exist

            let stage = PipelineShaderStageCreateInfo::new(compute_shader);

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

        Ok(Self {
            maximum_reducer,
            active_tonemapper: None,
            device,
            queue,
            mem_alloc,
            ds_alloc,
            cb_alloc,
            pipeline,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create maximum reducer:\n{0}")]
    MaximumReducer(#[from] maximum_reducer::Error),

    #[error("Load Shader Error:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Into Pipeline Layout Info Error:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },

    #[error("Create Pipeline Layout Error:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Create Pipeline Error:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),
}
