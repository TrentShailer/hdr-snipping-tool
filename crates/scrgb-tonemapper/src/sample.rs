use half::f16;
use thiserror::Error;
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::BufferCopy,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::HostAccessError,
    Validated,
};

use crate::ScrgbTonemapper;

impl ScrgbTonemapper {
    /// Sample the HDR capture at a given point.
    pub fn sample(&self, vk: &VulkanInstance, point: [u32; 2]) -> Result<[f32; 4], Error> {
        if point[0] >= self.display.size[0] || point[1] >= self.display.size[1] {
            return Err(Error::OutOfBounds);
        }

        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            8,
        )?;

        let offset = (point[1] * self.display.size[0] * 8) + point[0] * 8;

        let buffer_copy = BufferCopy {
            src_offset: offset as u64,
            dst_offset: 0,
            size: 8,
            ..Default::default()
        };

        copy_buffer_and_wait(
            vk,
            self.input_buffer.clone(),
            staging_buffer.clone(),
            copy_buffer::Region::SingleRegion(buffer_copy),
        )?;

        let data = &staging_buffer.read()?[..];

        let r: f32 = f16::from_le_bytes([data[0], data[1]]).into();
        let g: f32 = f16::from_le_bytes([data[2], data[3]]).into();
        let b: f32 = f16::from_le_bytes([data[4], data[5]]).into();
        let a: f32 = f16::from_le_bytes([data[6], data[7]]).into();

        Ok([r, g, b, a])
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Sample point was out of bounds.")]
    OutOfBounds,

    #[error("Failed to allocate buffer:\n{0:?}")]
    AllocateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0:?}")]
    BufferAccess(#[from] HostAccessError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] copy_buffer::Error),
}
