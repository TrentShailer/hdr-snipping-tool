use ash::vk::{
    AccessFlags2, Buffer, BufferCopy2, BufferUsageFlags, CopyBufferInfo2, DependencyInfo,
    DeviceMemory, MemoryPropertyFlags, PipelineStageFlags2,
};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

pub type VertexIndexBuffer = ((Buffer, DeviceMemory), (Buffer, DeviceMemory));

/// Creates device local vertex and index buffers
/// from a set of verticies and indicies.
#[instrument(skip_all, err)]
pub fn create_vertex_and_index_buffer<V: Copy>(
    vk: &VulkanInstance,
    verticies: &[V],
    indicies: &[u32],
) -> Result<VertexIndexBuffer, VulkanError> {
    // Vertex buffer
    let vertex_buffer_size = std::mem::size_of_val(verticies) as u64;

    let (vertex, vertex_memory) = vk.create_bound_buffer(
        vertex_buffer_size,
        BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
        MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let (vertex_staging, vertex_staging_memory) = vk.create_unbound_buffer(
        vertex_buffer_size,
        BufferUsageFlags::TRANSFER_SRC,
        MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        vk.write_to_memory(vertex_staging_memory, verticies, 0, vertex_buffer_size)?;
        vk.device
            .bind_buffer_memory(vertex_staging, vertex_staging_memory, 0)
            .map_err(|e| VulkanError::VkResult(e, "binding vertex staging memory"))?;
    }

    // Index buffer
    let index_buffer_size = std::mem::size_of_val(indicies) as u64;

    let (index, index_memory) = vk.create_bound_buffer(
        index_buffer_size,
        BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::TRANSFER_DST,
        MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let (index_staging, index_staging_memory) = vk.create_unbound_buffer(
        index_buffer_size,
        BufferUsageFlags::TRANSFER_SRC,
        MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        vk.write_to_memory(index_staging_memory, indicies, 0, index_buffer_size)?;
        vk.device
            .bind_buffer_memory(index_staging, index_staging_memory, 0)
            .map_err(|e| VulkanError::VkResult(e, "binding index staging memory"))?;
    }

    // Copy buffers

    vk.record_submit_command_buffer(vk.command_buffer, &[], &[], |device, command_buffer| {
        let memory_barriers = [
            VulkanInstance::buffer_memory_barrier()
                .dst_access_mask(AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(PipelineStageFlags2::TRANSFER)
                .buffer(vertex)
                .size(vertex_buffer_size),
            VulkanInstance::buffer_memory_barrier()
                .dst_access_mask(AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(PipelineStageFlags2::TRANSFER)
                .buffer(index)
                .size(index_buffer_size),
        ];

        let dependency_info = DependencyInfo::default().buffer_memory_barriers(&memory_barriers);

        unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

        // vertex copy
        let vertex_buffer_copy = BufferCopy2::default().size(vertex_buffer_size);
        let vertex_buffer_copy_regions = &[vertex_buffer_copy];

        let vertex_buffer_copy_info = CopyBufferInfo2::default()
            .src_buffer(vertex_staging)
            .dst_buffer(vertex)
            .regions(vertex_buffer_copy_regions);

        unsafe { device.cmd_copy_buffer2(command_buffer, &vertex_buffer_copy_info) }

        // index copy
        let index_buffer_copy = BufferCopy2::default().size(index_buffer_size);
        let index_buffer_copy_regions = &[index_buffer_copy];

        let index_buffer_copy_info = CopyBufferInfo2::default()
            .src_buffer(index_staging)
            .dst_buffer(index)
            .regions(index_buffer_copy_regions);

        unsafe { device.cmd_copy_buffer2(command_buffer, &index_buffer_copy_info) }
        Ok(())
    })?;

    unsafe {
        vk.device.destroy_buffer(vertex_staging, None);
        vk.device.destroy_buffer(index_staging, None);
        vk.device.free_memory(vertex_staging_memory, None);
        vk.device.free_memory(index_staging_memory, None);
    }

    Ok(((vertex, vertex_memory), (index, index_memory)))
}
