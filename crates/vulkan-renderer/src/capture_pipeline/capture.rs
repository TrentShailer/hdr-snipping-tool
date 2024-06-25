use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::{texture::Texture, VulkanInstance};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferExecError, CommandBufferUsage, CopyBufferInfo, PrimaryAutoCommandBuffer,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

use super::vertex::Vertex;

pub const VERTICIES: [Vertex; 4] = [
    Vertex {
        position: [-1.0, -1.0],
        uv: [0.0, 0.0],
    }, // TL
    Vertex {
        position: [1.0, -1.0],
        uv: [1.0, 0.0],
    }, // TR
    Vertex {
        position: [1.0, 1.0],
        uv: [1.0, 1.0],
    }, // BR
    Vertex {
        position: [-1.0, 1.0],
        uv: [0.0, 1.0],
    }, // BL
];

pub const INDICIES: [u32; 6] = [0, 1, 2, 2, 3, 0];

pub struct CaptureObject {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub capture: Option<Arc<Texture>>,
    pub capture_ds: Option<Arc<PersistentDescriptorSet>>,
}

impl CaptureObject {
    pub fn new(vk: &VulkanInstance, pipeline: Arc<GraphicsPipeline>) -> Result<Self, Error> {
        // verticies
        let vertex_staging = Buffer::from_iter(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            VERTICIES,
        )?;

        let vertex_buffer = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            VERTICIES.len() as u64,
        )?;

        // indicies
        let index_staging = Buffer::from_iter(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            INDICIES,
        )?;

        let index_buffer = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
            INDICIES.len() as u64,
        )?;

        // Copy from staging to buffer
        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        builder.copy_buffer(CopyBufferInfo::buffers(
            vertex_staging.clone(),
            vertex_buffer.clone(),
        ))?;
        builder.copy_buffer(CopyBufferInfo::buffers(
            index_staging.clone(),
            index_buffer.clone(),
        ))?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFence)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            capture: None,
            capture_ds: None,
        })
    }

    pub fn load_capture(
        &mut self,
        vk: &VulkanInstance,
        texture: Arc<Texture>,
    ) -> Result<(), Error> {
        let ds_layout = self.pipeline.layout().set_layouts()[0].clone();

        let descriptor_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, texture.sampler.clone()),
                WriteDescriptorSet::image_view(1, texture.image_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        self.capture = Some(texture);
        self.capture_ds = Some(descriptor_set);

        Ok(())
    }

    pub fn unload_capture(&mut self) {
        self.capture = None;
        self.capture_ds = None;
    }

    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
    ) -> Result<(), Box<ValidationError>> {
        let capture_ds = match self.capture_ds.as_ref() {
            Some(v) => v,
            None => return Ok(()),
        };

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                capture_ds.clone(),
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create buffers:\n{0:?}")]
    CreateBuffers(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0:?}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] Box<ValidationError>),

    #[error("Failed to signal fence:\n{0:?}")]
    SignalFence(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
