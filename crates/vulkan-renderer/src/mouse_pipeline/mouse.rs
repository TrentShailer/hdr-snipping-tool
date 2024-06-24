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

const LEFT_OR_TOP_FLAG: u32 = 0b00000000_00000000_00000000_00000100;
const POSITIVE_SHIFT_FLAG: u32 = 0b00000000_00000000_00000000_00000010;
const VERTICAL_FLAG: u32 = 0b00000000_00000000_00000000_00000001;
const NO_FLAGS: u32 = 0b00000000_00000000_00000000_00000000;

/* Vertex indicies
            .0.1

    .6					.4
    .7					.5

            .2.3
*/

pub const VERTICIES: [Vertex; 8] = [
    Vertex {
        position: [0.0, -1.0],
        color: [255, 255, 255, 255],
        flags: LEFT_OR_TOP_FLAG | VERTICAL_FLAG,
    },
    Vertex {
        position: [0.0, -1.0],
        color: [255, 255, 255, 255],
        flags: LEFT_OR_TOP_FLAG | POSITIVE_SHIFT_FLAG | VERTICAL_FLAG,
    },
    Vertex {
        position: [0.0, 1.0],
        color: [255, 255, 255, 255],
        flags: VERTICAL_FLAG,
    },
    Vertex {
        position: [0.0, 1.0],
        color: [255, 255, 255, 255],
        flags: POSITIVE_SHIFT_FLAG | VERTICAL_FLAG,
    },
    //
    Vertex {
        position: [1.0, 0.0],
        color: [255, 255, 255, 255],
        flags: NO_FLAGS,
    },
    Vertex {
        position: [1.0, 0.0],
        color: [255, 255, 255, 255],
        flags: POSITIVE_SHIFT_FLAG,
    },
    Vertex {
        position: [-1.0, 0.0],
        color: [255, 255, 255, 255],
        flags: LEFT_OR_TOP_FLAG,
    },
    Vertex {
        position: [-1.0, 0.0],
        color: [255, 255, 255, 255],
        flags: LEFT_OR_TOP_FLAG | POSITIVE_SHIFT_FLAG,
    },
];

pub const INDICIES: [u32; 12] = [
    0, 1, 2, 2, 1, 3, //
    4, 5, 6, 6, 5, 7, //
];

pub struct Mouse {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub line_size: f32,
}

impl Mouse {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        line_size: f32,
    ) -> Result<Self, Error> {
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
            line_size,
        })
    }

    pub fn render(
        &self,
        command_buffer: &mut AutoCommandBufferBuilder<
            PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>,
            Arc<StandardCommandBufferAllocator>,
        >,
        mouse_position: [u32; 2], // xy
        window_size: PhysicalSize<u32>,
    ) -> Result<(), Box<ValidationError>> {
        // Convert position to -1.0, 1.0
        let x = (mouse_position[0] as f32 / window_size.width as f32) * 2.0 - 1.0;
        let y = (mouse_position[1] as f32 / window_size.height as f32) * 2.0 - 1.0;
        let mouse_position = [x, y];

        // Calcualte line size
        let line_size_x = self.line_size / window_size.width as f32;
        let line_size_y = self.line_size / window_size.height as f32;

        let line_size = [line_size_x, line_size_y];

        command_buffer
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?
            .bind_index_buffer(self.index_buffer.clone())?
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                PushConstants {
                    mouse_position,
                    line_size,
                },
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
