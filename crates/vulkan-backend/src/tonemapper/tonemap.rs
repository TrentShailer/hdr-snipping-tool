use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyBufferToImageInfo,
    },
    image::Image,
    pipeline::Pipeline,
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

use crate::VulkanInstance;

use super::{shader::Config, Tonemapper};

#[derive(Debug, Error)]
pub enum Error {
    #[error("No capture loaded")]
    NotLoaded,

    #[error("New Command Buffer Error:\n{0:?}")]
    NewCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Bind Pipline Error:\n{0}")]
    BindPipeline(#[source] Box<ValidationError>),

    #[error("Bind Descriptor Sets Error:\n{0}")]
    BindDescriptorSets(#[source] Box<ValidationError>),

    #[error("Bind Push Constants Error:\n{0}")]
    BindPushConstants(#[source] Box<ValidationError>),

    #[error("Dispatch Error:\n{0}")]
    Dispatch(#[source] Box<ValidationError>),

    #[error("Build Command Buffer Error:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Exec Error:\n{0}")]
    Exec(#[from] CommandBufferExecError),

    #[error("Signal Fence Error:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Await Fence Error:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),

    #[error("Copy Buffer Error:\n{0}")]
    CopyBuffer(#[source] Box<ValidationError>),

    #[error("Write Config Error:\n{0}")]
    WriteConfig(#[from] HostAccessError),
}

impl Tonemapper {
    pub fn tonemap(
        &mut self,
        vulkan: &VulkanInstance,
        result_image: Arc<Image>,
    ) -> Result<(), Error> {
        let active_tonemapper = match self.active_tonemapper.as_mut() {
            Some(v) => v,
            None => return Err(Error::NotLoaded),
        };

        *active_tonemapper.config_buffer.write()? = Config {
            input_length: active_tonemapper.input_size,
            input_width: active_tonemapper.capture_size.width,
            input_height: active_tonemapper.capture_size.height,
            maximum: active_tonemapper.maximum,
            alpha: active_tonemapper.alpha,
            gamma: active_tonemapper.gamma,
        };

        let workgroup_x = active_tonemapper.capture_size.width.div_ceil(32);
        let workgroup_y = active_tonemapper.capture_size.height.div_ceil(32);

        let mut builder = AutoCommandBufferBuilder::primary(
            &vulkan.allocators.command,
            vulkan.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::NewCommandBuffer)?;

        builder
            .bind_pipeline_compute(self.pipeline.clone())
            .map_err(Error::BindPipeline)?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0,
                active_tonemapper.descriptor_set_0.clone(),
            )
            .map_err(Error::BindDescriptorSets)?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                1,
                active_tonemapper.descriptor_set_1.clone(),
            )
            .map_err(Error::BindDescriptorSets)?
            .dispatch([workgroup_x, workgroup_y, 1])
            .map_err(Error::Dispatch)?
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                active_tonemapper.output_buffer.clone(),
                result_image,
            ))
            .map_err(Error::CopyBuffer)?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vulkan.device.clone())
            .then_execute(vulkan.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        Ok(())
    }
}
