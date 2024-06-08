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
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    Validated, VulkanError,
};

use crate::VulkanInstance;

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "shaders/compute/tonemap.spv"}
}

pub struct Tonemapper {
    maximum_reducer: MaximumReducer,
    active_tonemapper: Option<ActiveTonemapper>,
    pipeline: Arc<ComputePipeline>,
}

impl Tonemapper {
    pub fn new(instance: &VulkanInstance) -> Result<Self, Error> {
        let maximum_reducer =
            MaximumReducer::new(instance.device.clone(), instance.allocators.clone())?;

        let pipeline = {
            let compute_shader = shader::load(instance.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap(); // Only none if 'main' doesn't exist

            let stage = PipelineShaderStageCreateInfo::new(compute_shader);

            let layout = PipelineLayout::new(
                instance.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(instance.device.clone())
                    .map_err(|e| Error::CreatePipelineLayoutInfo {
                        set_num: e.set_num,
                        error: e.error,
                    })?,
            )
            .map_err(Error::CreatePipelineLayout)?;

            ComputePipeline::new(
                instance.device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .map_err(Error::CreatePipeline)?
        };

        Ok(Self {
            maximum_reducer,
            active_tonemapper: None,
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
