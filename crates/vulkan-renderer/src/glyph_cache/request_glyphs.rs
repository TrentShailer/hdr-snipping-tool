use fontdue::layout::GlyphRasterConfig;
use smallvec::SmallVec;
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, BufferImageCopy, CommandBufferExecError, CommandBufferUsage,
        CopyBufferToImageInfo,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    sync::{self, GpuFuture, HostAccessError},
    Validated, ValidationError, VulkanError,
};

use super::{GlyphCache, GlyphData, ATLAS_DIM};

impl GlyphCache {
    /// Request the glyph data for a set of glyphs from the renderer.
    ///
    /// This will try to fetch them from the map if the glyph is present in the atlas.
    ///
    /// If the glyph is not present in the atlas, it is rasterised and then written to the atlas.
    ///
    /// The requested glyphs MUST be requested with the same font size as the text renderer.
    pub fn request_glyphs(
        &mut self,
        vk: &VulkanInstance,
        glyphs: &[GlyphRasterConfig],
    ) -> Result<Vec<GlyphData>, Error> {
        let mut glyph_data = Vec::with_capacity(glyphs.len());

        let mut bitmap_positions = vec![];
        let mut bitmap_buffer: Vec<u8> = vec![];
        let mut glyph_count = 0;

        let glyph_size = self.glyph_dim * self.glyph_dim;

        // Get glyph data for each glyph, rasterising new glyphs as neccecary
        for glyph in glyphs {
            let maybe_data = self.glyph_map.get(glyph);
            if let Some(data) = maybe_data {
                glyph_data.push(data.to_owned());
                continue;
            }

            let (metrics, bitmap) = self.font.rasterize_config(glyph.to_owned());

            // Allocate space for this bitmap
            bitmap_buffer.append(&mut vec![0; glyph_size]);

            // Copy bitmap into bitmap data
            let buffer_start = glyph_count * glyph_size;

            for row in 0..metrics.height {
                let buffer_row_start = buffer_start + (row * self.glyph_dim);
                let buffer_row_end = buffer_row_start + metrics.width;

                let bitmap_row_start = row * metrics.width;
                let bitmap_row_end = bitmap_row_start + metrics.width;

                bitmap_buffer[buffer_row_start..buffer_row_end]
                    .copy_from_slice(&bitmap[bitmap_row_start..bitmap_row_end]);
            }

            bitmap_positions.push(self.atlas_position);

            let new_glyph_data = GlyphData {
                metrics,
                index: self.atlas_position,
            };
            self.glyph_map.insert(glyph.to_owned(), new_glyph_data);
            glyph_data.push(new_glyph_data);

            // advace position
            glyph_count += 1;

            if self.atlas_position.x + 1 == ATLAS_DIM {
                self.atlas_position.x = 0;
                self.atlas_position.y += 1;
            } else {
                self.atlas_position.x += 1;
            }

            if self.atlas_position.y >= ATLAS_DIM {
                return Err(Error::AtlasOverflow);
            }
        }

        if glyph_count == 0 {
            return Ok(glyph_data);
        }

        // Write bitmap data to image
        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            (glyph_count * glyph_size) as u64,
        )?;
        staging_buffer.write()?.copy_from_slice(&bitmap_buffer);

        let mut builder = AutoCommandBufferBuilder::primary(
            &vk.allocators.command,
            vk.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .map_err(Error::CreateCommandBuffer)?;

        let mut copy_info =
            CopyBufferToImageInfo::buffer_image(staging_buffer.clone(), self.atlas.clone());

        let regions = (0..glyph_count)
            .map(|index| {
                let position = bitmap_positions[index];
                let x_offset = position.x * self.glyph_dim;
                let y_offset = position.y * self.glyph_dim;

                BufferImageCopy {
                    buffer_offset: (index * glyph_size) as u64,
                    buffer_row_length: self.glyph_dim as u32,
                    buffer_image_height: (self.glyph_dim * glyph_count) as u32,
                    image_subresource: self.atlas.subresource_layers(),
                    image_offset: [x_offset as u32, y_offset as u32, 0],
                    image_extent: [self.glyph_dim as u32, self.glyph_dim as u32, 1],
                    ..Default::default()
                }
            })
            .collect::<SmallVec<[BufferImageCopy; 1]>>();

        copy_info.regions = regions;

        builder.copy_buffer_to_image(copy_info)?;

        let command_buffer = builder.build().map_err(Error::BuildCommandBuffer)?;
        let future = sync::now(vk.device.clone())
            .then_execute(vk.queue.clone(), command_buffer)
            .map_err(Error::ExecCommandBuffer)?
            .then_signal_fence_and_flush()
            .map_err(Error::SignalFenceAndFlush)?;
        future.wait(None).map_err(Error::AwaitFence)?;

        Ok(glyph_data)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Font atlas ran out of space")]
    AtlasOverflow,

    #[error("Failed to create buffer:\n{0:?}")]
    CreateBuffers(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0}")]
    AccessBuffer(#[from] HostAccessError),

    #[error("Failed to create command buffer:\n{0:?}")]
    CreateCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to build command buffer:\n{0:?}")]
    BuildCommandBuffer(#[source] Validated<VulkanError>),

    #[error("Failed to execute command buffer:\n{0:?}")]
    ExecCommandBuffer(#[from] CommandBufferExecError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] Box<ValidationError>),

    #[error("Failed to signal fence and flush:\n{0:?}")]
    SignalFenceAndFlush(#[source] Validated<VulkanError>),

    #[error("Failed to await fence:\n{0:?}")]
    AwaitFence(#[source] Validated<VulkanError>),
}
