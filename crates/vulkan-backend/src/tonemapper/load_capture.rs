use half::f16;
use thiserror::Error;

use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::Pipeline,
    sync::HostAccessError,
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;

use crate::VulkanInstance;

use super::{active_tonemapper::ActiveTonemapper, maximum_reducer, shader, Tonemapper};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to find maximum:\n{0}")]
    Maximum(#[from] maximum_reducer::reduce::Error),

    #[error("Create Buffer Error:\n{0:?}")]
    CreateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Create Descriptor Set Error:\n{0:?}")]
    CreateDescriptorSet(#[from] Validated<VulkanError>),

    #[error("Buffer Access Error:\n{0}")]
    BufferAccess(#[from] HostAccessError),
}

impl Tonemapper {
    pub fn load_capture(
        &mut self,
        vulkan: &VulkanInstance,
        raw_capture: &[u8],
        default_alpha: f16,
        default_gamma: f16,
        size: PhysicalSize<u32>,
    ) -> Result<(), Error> {
        let maximum = self.maximum_reducer.find_maximum(&vulkan, raw_capture)?;

        let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vulkan.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            raw_capture.len() as u64,
        )?;

        let output_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vulkan.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
            (size.width * size.height * 4) as u64,
        )?;

        let config_buffer: Subbuffer<shader::Config> = Buffer::new_sized(
            vulkan.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        )?;

        let layout_0 = &self.pipeline.layout().set_layouts()[0];
        let descriptor_set_0 = PersistentDescriptorSet::new(
            &vulkan.allocators.descriptor,
            layout_0.clone(),
            [
                WriteDescriptorSet::buffer(0, input_buffer.clone()),
                WriteDescriptorSet::buffer(1, output_buffer.clone()),
            ],
            [],
        )?;

        let layout_1 = &self.pipeline.layout().set_layouts()[1];
        let descriptor_set_1 = PersistentDescriptorSet::new(
            &vulkan.allocators.descriptor,
            layout_1.clone(),
            [WriteDescriptorSet::buffer(0, config_buffer.clone())],
            [],
        )?;

        input_buffer.write()?[0..raw_capture.len()].copy_from_slice(raw_capture);

        let active_tonemapper = ActiveTonemapper {
            maximum,
            capture_size: size,
            input_size: raw_capture.len() as u32 / 2,
            input_buffer,
            output_buffer,
            config_buffer,
            descriptor_set_0,
            descriptor_set_1,
            alpha: default_alpha,
            gamma: default_gamma,
        };

        self.active_tonemapper = Some(active_tonemapper);

        Ok(())
    }
}
