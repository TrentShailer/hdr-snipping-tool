pub mod tonemap;

use std::sync::Arc;

use half::f16;
use shader::Config;
use thiserror::Error;
use vulkan_instance::{texture::Texture, VulkanInstance};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo, layout::PipelineDescriptorSetLayoutCreateInfo,
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::HostAccessError,
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;

use crate::{
    find_maximum::{self, find_maximum},
    Tonemapper,
};

pub mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/tonemap.spv"}
}

impl Tonemapper {
    pub fn new(
        vk: &VulkanInstance,
        texture: Arc<Texture>,
        bytes: &[u8],
        size: PhysicalSize<u32>,
        alpha: f16,
        gamma: f16,
    ) -> Result<Self, Error> {
        let pipeline = {
            let compute_shader = shader::load(vk.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap();

            let stage = PipelineShaderStageCreateInfo::new(compute_shader);

            let layout = PipelineLayout::new(
                vk.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(vk.device.clone())
                    .map_err(|e| Error::CreatePipelineLayoutInfo {
                        set_num: e.set_num,
                        error: e.error,
                    })?,
            )
            .map_err(Error::CreatePipelineLayout)?;

            ComputePipeline::new(
                vk.device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .map_err(Error::CreatePipeline)?
        };

        let maximum = find_maximum(&vk, bytes)?;

        let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            bytes.len() as u64,
        )?;

        let config = Config {
            alpha,
            gamma,
            maximum,
            input_width: size.width,
            input_height: size.height,
        };

        let config_buffer: Subbuffer<shader::Config> = Buffer::from_data(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            config,
        )?;

        let io_layout = &pipeline.layout().set_layouts()[0];
        let io_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            io_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, input_buffer.clone()),
                WriteDescriptorSet::image_view(1, texture.image_view.clone()),
            ],
            [],
        )
        .map_err(Error::Descriptor)?;

        let config_layout = &pipeline.layout().set_layouts()[1];
        let config_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            config_layout.clone(),
            [WriteDescriptorSet::buffer(0, config_buffer.clone())],
            [],
        )
        .map_err(Error::Descriptor)?;

        input_buffer.write()?.copy_from_slice(bytes);

        Ok(Self {
            pipeline,
            config,
            input_buffer,
            config_buffer,
            io_set,
            config_set,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to find maximum:\n{0}")]
    Maximum(#[from] find_maximum::Error),

    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to create pipeline layout info:\nSet {set_num}\n{error:?}")]
    CreatePipelineLayoutInfo {
        set_num: u32,
        error: Validated<VulkanError>,
    },

    #[error("Failedd to create pipeline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Failed to allocate buffer:\n{0:?}")]
    AllocateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    Descriptor(#[source] Validated<VulkanError>),

    #[error("Failed to access buffer:\n{0:?}")]
    BufferAccess(#[from] HostAccessError),
}
