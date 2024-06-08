use half::f16;
use thiserror::Error;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage},
    pipeline::Pipeline,
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

use crate::VulkanInstance;

use super::{MaximumReducer, MAXIMUM_INPUT_BUFFER_SIZE};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Input is larger than buffer.")]
    InputOutOfBounds,

    #[error("Input has odd number of bytes.")]
    CorruptInput,

    #[error("Buffer Access Error:\n{0}")]
    BufferAccessError(#[from] HostAccessError),

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
}

impl MaximumReducer {
    pub fn find_maximum(&self, instance: &VulkanInstance, bytes: &[u8]) -> Result<f16, Error> {
        // Basic data validity checks
        if bytes.len() as u64 > MAXIMUM_INPUT_BUFFER_SIZE {
            return Err(Error::InputOutOfBounds);
        }
        if bytes.len() % 2 != 0 {
            return Err(Error::CorruptInput);
        }

        let mut input_length = bytes.len() as u32 / 2;
        let mut output_length = (bytes.len() as u32 / 2).div_ceil(self.compute_blocksize);

        self.input_buffer.write()?[..bytes.len()].copy_from_slice(bytes);

        // While there is multiple candidates, do a pass
        // and swap the input and output buffer
        let mut use_inverse_set = false;
        while input_length > 1 {
            let workgroup_count = output_length;

            let mut builder = AutoCommandBufferBuilder::primary(
                &instance.allocators.command,
                instance.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .map_err(Error::NewCommandBuffer)?;

            let set = if use_inverse_set {
                self.inverse_descriptor_set.clone()
            } else {
                self.descriptor_set.clone()
            };

            builder
                .bind_pipeline_compute(self.pipeline.clone())
                .map_err(Error::BindPipeline)?
                .bind_descriptor_sets(
                    vulkano::pipeline::PipelineBindPoint::Compute,
                    self.pipeline.layout().clone(),
                    0,
                    set,
                )
                .map_err(Error::BindDescriptorSets)?
                .push_constants(self.pipeline.layout().clone(), 0, input_length)
                .map_err(Error::BindPushConstants)?
                .dispatch([workgroup_count, 1, 1])
                .map_err(Error::Dispatch)?;

            let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
            let future = sync::now(instance.device.clone())
                .then_execute(instance.queue.clone(), command_buffer)?
                .then_signal_fence_and_flush()
                .map_err(Error::SignalFence)?;
            future.wait(None).map_err(Error::AwaitFence)?;

            use_inverse_set = !use_inverse_set;
            input_length = output_length;
            output_length = input_length.div_ceil(self.compute_blocksize);
        }

        let result_buffer = if use_inverse_set {
            self.output_buffer.clone()
        } else {
            self.input_buffer.clone()
        };

        let reader = &result_buffer.read().map_err(Error::BufferAccessError)?;
        let maximum = f16::from_le_bytes([reader[0], reader[1]]);

        log::info!("maximum: {:.2}", maximum);

        Ok(maximum)
    }
}
