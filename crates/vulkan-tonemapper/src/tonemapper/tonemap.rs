use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage},
    pipeline::Pipeline,
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

use crate::tonemapper::debug;

use super::Tonemapper;

impl Tonemapper {
    pub fn tonemap(&mut self, vk: &VulkanInstance) -> Result<(), Error> {
        let workgroup_x = self.config.input_width.div_ceil(32);
        let workgroup_y = self.config.input_height.div_ceil(32);

        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        debug::maybe_reset(&self, &mut builder);
        debug::maybe_record_timestamp(&self, &mut builder, 0, sync::PipelineStage::TopOfPipe);

        builder
            .bind_pipeline_compute(self.pipeline.clone())?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0,
                self.io_set.clone(),
            )?
            .push_constants(self.pipeline.layout().clone(), 0, self.config)?
            .dispatch([workgroup_x, workgroup_y, 1])?;

        debug::maybe_record_timestamp(&self, &mut builder, 1, sync::PipelineStage::BottomOfPipe);

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        debug::maybe_log_tonemap_time(vk, &self);

        Ok(())
    }
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

    #[error("Failed to signal fence:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),
}
