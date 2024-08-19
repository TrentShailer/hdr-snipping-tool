use ash::{
    util::Align,
    vk::{
        AccessFlags2, Buffer, BufferCopy2, BufferMemoryBarrier2, BufferUsageFlags, CopyBufferInfo2,
        DependencyInfo, DeviceMemory, MemoryMapFlags, MemoryPropertyFlags, PipelineStageFlags2,
        QUEUE_FAMILY_IGNORED,
    },
};
use thiserror::Error;
use vulkan_instance::{record_submit_command_buffer, VulkanInstance};

pub type VertexIndexBuffer = ((Buffer, DeviceMemory), (Buffer, DeviceMemory));

/// Creates device local vertex and index buffers
/// from a set of verticies and indicies.
pub fn create_vertex_and_index_buffer<V: Copy>(
    vk: &VulkanInstance,
    verticies: &[V],
    indicies: &[u32],
) -> Result<VertexIndexBuffer, Error> {
    // Vertex buffer
    let vertex_size = std::mem::size_of::<V>();
    let vertex_buffer_size = (verticies.len() * vertex_size) as u64;

    let (vertex, vertex_memory) = vk
        .create_bound_buffer(
            vertex_buffer_size,
            BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .map_err(|e| Error::Vulkan(e, "creating vertex buffer"))?;

    let (vertex_staging, vertex_staging_memory) = vk
        .create_unbound_buffer(
            vertex_buffer_size,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
        .map_err(|e| Error::Vulkan(e, "creating vertex staging buffer"))?;

    unsafe {
        let staging_ptr = vk
            .device
            .map_memory(
                vertex_staging_memory,
                0,
                vertex_buffer_size,
                MemoryMapFlags::empty(),
            )
            .map_err(|e| Error::Vulkan(e, "mapping vertex staging memory"))?;

        let mut staging_slice = Align::new(
            staging_ptr,
            std::mem::align_of::<V>() as u64,
            vertex_buffer_size,
        );
        staging_slice.copy_from_slice(verticies);

        vk.device.unmap_memory(vertex_staging_memory);
        vk.device
            .bind_buffer_memory(vertex_staging, vertex_staging_memory, 0)
            .map_err(|e| Error::Vulkan(e, "binding vertex staging memory"))?;
    }

    // Index buffer
    let index_size = std::mem::size_of::<u32>();
    let index_buffer_size = (indicies.len() * index_size) as u64;

    let (index, index_memory) = vk
        .create_bound_buffer(
            index_buffer_size,
            BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .map_err(|e| Error::Vulkan(e, "creating index buffer"))?;

    let (index_staging, index_staging_memory) = vk
        .create_unbound_buffer(
            index_buffer_size,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
        .map_err(|e| Error::Vulkan(e, "creating index staging buffer"))?;

    unsafe {
        let staging_ptr = vk
            .device
            .map_memory(
                index_staging_memory,
                0,
                index_buffer_size,
                MemoryMapFlags::empty(),
            )
            .map_err(|e| Error::Vulkan(e, "mapping index staging memory"))?;

        let mut staging_slice = Align::new(
            staging_ptr,
            std::mem::align_of::<u32>() as u64,
            index_buffer_size,
        );
        staging_slice.copy_from_slice(indicies);

        vk.device.unmap_memory(index_staging_memory);
        vk.device
            .bind_buffer_memory(index_staging, index_staging_memory, 0)
            .map_err(|e| Error::Vulkan(e, "binding index staging memory"))?;
    }

    // Copy buffers

    vk.record_submit_command_buffer(
        vk.command_buffer,
        vk.fence,
        &[],
        &[],
        |device, command_buffer| {
            let memory_barriers = [
                BufferMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::TRANSFER,
                    dst_access_mask: AccessFlags2::MEMORY_WRITE,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    buffer: vertex,
                    offset: 0,
                    size: vertex_buffer_size,
                    ..Default::default()
                },
                BufferMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::TRANSFER,
                    dst_access_mask: AccessFlags2::MEMORY_WRITE,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    buffer: index,
                    offset: 0,
                    size: index_buffer_size,
                    ..Default::default()
                },
            ];

            let dependency_info =
                DependencyInfo::default().buffer_memory_barriers(&memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

            // vertex copy
            let vertex_buffer_copy = BufferCopy2 {
                src_offset: 0,
                dst_offset: 0,
                size: vertex_buffer_size,
                ..Default::default()
            };
            let vertex_buffer_copy_regions = &[vertex_buffer_copy];

            let vertex_buffer_copy_info = CopyBufferInfo2::default()
                .src_buffer(vertex_staging)
                .dst_buffer(vertex)
                .regions(vertex_buffer_copy_regions);

            unsafe { device.cmd_copy_buffer2(command_buffer, &vertex_buffer_copy_info) }

            // index copy
            let index_buffer_copy = BufferCopy2 {
                src_offset: 0,
                dst_offset: 0,
                size: index_buffer_size,
                ..Default::default()
            };
            let index_buffer_copy_regions = &[index_buffer_copy];

            let index_buffer_copy_info = CopyBufferInfo2::default()
                .src_buffer(index_staging)
                .dst_buffer(index)
                .regions(index_buffer_copy_regions);

            unsafe { device.cmd_copy_buffer2(command_buffer, &index_buffer_copy_info) }
            Ok(())
        },
    )?;

    Ok(((vertex, vertex_memory), (index, index_memory)))
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] record_submit_command_buffer::Error),
}
