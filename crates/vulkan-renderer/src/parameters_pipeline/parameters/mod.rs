pub mod render;
pub mod set_text;
pub mod text_renderer;

use std::sync::Arc;

use fontdue::layout::Layout;
use render::TEXT_OFFSET;
use text_renderer::{TextRenderer, FONT_SIZE};
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferExecError, CommandBufferUsage, CopyBufferInfo,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{GraphicsPipeline, Pipeline},
    sync::{self, GpuFuture},
    Validated, ValidationError, VulkanError,
};

use crate::renderer::units::{LogicalPosition, LogicalScale};

use super::vertex::{InstanceData, Vertex};

pub const INDICIES: [u32; 6] = [0, 1, 2, 2, 3, 0];

pub struct Parameters {
    pub vertex_buffer: Subbuffer<[Vertex]>,
    pub index_buffer: Subbuffer<[u32]>,
    pub instance_buffer: Subbuffer<[InstanceData]>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub instances: u32,
    //
    pub text_renderer: TextRenderer,
    pub layout: Layout,
    pub atlas_ds: Arc<PersistentDescriptorSet>,
    pub text_right: f32,
}

impl Parameters {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        instance_capacity: u32,
    ) -> Result<Self, Error> {
        let mut layout = Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
        let text_renderer = TextRenderer::new(&vk, &mut layout)?;

        let uv_x = FONT_SIZE / text_renderer.atlas.extent()[1] as f32;
        let uv_y = FONT_SIZE / text_renderer.atlas.extent()[0] as f32;

        // verticies
        let verticies: [Vertex; 4] = [
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 0.0],
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                uv: [uv_x, 0.0],
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                uv: [uv_x, uv_y],
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, uv_y],
            }, // BL
        ];

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
            verticies,
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
            verticies.len() as u64,
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

        let instance_buffer: Subbuffer<[InstanceData]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            instance_capacity as u64,
        )?;

        let atlas_ds_layout = pipeline.layout().set_layouts()[0].clone();

        let atlas_ds = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            atlas_ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, text_renderer.atlas_sampler.clone()),
                WriteDescriptorSet::image_view(1, text_renderer.atlas_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            pipeline,
            instances: 0,
            //
            text_renderer,
            layout,
            atlas_ds,
            text_right: 0.0,
        })
    }

    pub fn get_position_size(
        &self,
        mouse_position: [u32; 2],
        window_size: [u32; 2],
    ) -> (LogicalPosition, LogicalScale) {
        let text_offset = LogicalScale::from_f32x2([TEXT_OFFSET, TEXT_OFFSET], window_size);

        let text_scale =
            LogicalScale::from_f32x2([self.text_right, self.layout.height()], window_size);

        let obscured =
            mouse_position[0] > 2 * window_size[0] / 3 && mouse_position[1] < window_size[1] / 3;

        let text_position = if obscured {
            LogicalPosition::new(-1.0 + text_offset.x, -1.0 + text_offset.y)
        } else {
            let r = 1.0 - text_scale.x * 2.0;
            LogicalPosition::new(r - text_offset.x, -1.0 + text_offset.y)
        };

        (text_position, text_scale)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create buffer:\n{0:?}")]
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

    #[error("Failed to create atlas:\n{0}")]
    Atlas(#[from] text_renderer::Error),
}
