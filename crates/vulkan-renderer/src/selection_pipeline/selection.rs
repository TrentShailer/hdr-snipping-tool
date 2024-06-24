use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferExecError, CommandBufferUsage, CopyBufferInfo, PrimaryAutoCommandBuffer,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};
use winit::dpi::PhysicalSize;

use super::{vertex::Vertex, vertex_shader::PushConstants};

const LOCKED_FLAG: u32 = 0b00000000_00000000_00000000_00000100;
const TOP_FLAG: u32 = 0b00000000_00000000_00000000_00000010;
const LEFT_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

/* Vertex indicies
    .0				.2
        .1		.3

        .7		.5
    .6				.4
*/

pub const VERTICIES: [Vertex; 8] = [
    Vertex {
        position: [-1.0, -1.0],
        color: [0, 0, 0, 128],
        flags: LOCKED_FLAG | TOP_FLAG | LEFT_FLAG,
    }, // TL
    Vertex {
        position: [0.0, 0.0],
        color: [0, 0, 0, 128],
        flags: TOP_FLAG | LEFT_FLAG,
    }, // CTL
    Vertex {
        position: [1.0, -1.0],
        color: [0, 0, 0, 128],
        flags: LOCKED_FLAG | TOP_FLAG,
    }, // TR
    Vertex {
        position: [0.0, 0.0],
        color: [0, 0, 0, 128],
        flags: TOP_FLAG,
    }, // CTR
    Vertex {
        position: [1.0, 1.0],
        color: [0, 0, 0, 128],
        flags: LOCKED_FLAG,
    }, // BR
    Vertex {
        position: [0.0, 0.0],
        color: [0, 0, 0, 128],
        flags: NO_FLAGS,
    }, // CBR
    Vertex {
        position: [-1.0, 1.0],
        color: [0, 0, 0, 128],
        flags: LOCKED_FLAG | LEFT_FLAG,
    }, // BL
    Vertex {
        position: [0.0, 0.0],
        color: [0, 0, 0, 128],
        flags: LEFT_FLAG,
    }, // CBL
];

pub const INDICIES: [u32; 24] = [
    0, 2, 1, 1, 2, 3, //
    3, 2, 5, 4, 5, 2, //
    7, 5, 4, 6, 7, 4, //
    1, 7, 6, 0, 1, 6, //
];

pub struct Selection {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
}

impl Selection {
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
        })
    }

    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        selection: [u32; 4], // ltrb
        window_size: PhysicalSize<u32>,
    ) -> Result<(), Box<ValidationError>> {
        // Convert seleciton to -1.0, 1.0
        let l = (selection[0] as f32 / window_size.width as f32) * 2.0 - 1.0;
        let t = (selection[1] as f32 / window_size.height as f32) * 2.0 - 1.0;
        let r = (selection[2] as f32 / window_size.width as f32) * 2.0 - 1.0;
        let b = (selection[3] as f32 / window_size.height as f32) * 2.0 - 1.0;
        let selection = [l, t, r, b];

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants { selection },
            )?
            .draw_indexed(self.index_buffer.len() as u32, 1, 0, 0, 0)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create buffers:\n{0}")]
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
