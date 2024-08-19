use std::sync::Arc;

use ash::{
    vk::{
        AccessFlags2, BufferCopy2, BufferMemoryBarrier2, BufferUsageFlags, CommandBuffer,
        CommandBufferAllocateInfo, CommandBufferLevel, CopyBufferInfo2, DependencyInfo, Fence,
        FenceCreateFlags, FenceCreateInfo, ImageView, MemoryMapFlags, MemoryPropertyFlags,
        PhysicalDeviceProperties2, PhysicalDeviceSubgroupProperties, PipelineStageFlags2,
        Semaphore, SemaphoreCreateInfo, QUEUE_FAMILY_IGNORED,
    },
    Device,
};
use buffer_pass::BufferPass;
use half::f16;
use source_pass::SourcePass;
use thiserror::Error;
use tracing::info_span;
use vulkan_instance::VulkanInstance;

mod buffer_pass;
mod source_pass;

pub const MAXIMUM_SUBMISSIONS: usize = 3;

/// Object that facilitates finding the maximum value of an input image.
pub struct Maximum {
    device: Arc<Device>,

    command_buffers: Vec<CommandBuffer>,
    fences: Vec<Fence>,
    semaphores: Vec<Semaphore>,

    source_pass: SourcePass,
    buffer_pass: BufferPass,
}

impl Maximum {
    pub fn new(vk: &VulkanInstance) -> Result<Self, Error> {
        let _span = info_span!("Maximum::new").entered();
        // create command buffers
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .command_buffer_count(MAXIMUM_SUBMISSIONS as u32)
            .command_pool(vk.command_buffer_pool)
            .level(CommandBufferLevel::PRIMARY);
        let command_buffers = unsafe {
            vk.device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| Error::Vulkan(e, "allocating command buffers"))?
        };

        // create fences
        let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED);
        let fences: Vec<Fence> = (0..MAXIMUM_SUBMISSIONS)
            .into_iter()
            .map(|_| unsafe {
                vk.device
                    .create_fence(&fence_create_info, None)
                    .map_err(|e| Error::Vulkan(e, "creating fence"))
            })
            .collect::<Result<_, Error>>()?;

        // create semaphores
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let semaphores: Vec<Semaphore> = (0..MAXIMUM_SUBMISSIONS)
            .into_iter()
            .map(|_| unsafe {
                vk.device
                    .create_semaphore(&semaphore_create_info, None)
                    .map_err(|e| Error::Vulkan(e, "creating semaphore"))
            })
            .collect::<Result<_, Error>>()?;

        let source_pass = SourcePass::new(vk)?;
        let buffer_pass = BufferPass::new(vk)?;

        Ok(Self {
            device: vk.device.clone(),
            command_buffers,
            fences,
            semaphores,
            source_pass,
            buffer_pass,
        })
    }

    pub fn find_maximum(
        &self,
        vk: &VulkanInstance,
        source: ImageView,
        source_size: [u32; 2],
    ) -> Result<f16, Error> {
        let _span = info_span!("find_maximum").entered();

        let subgroup_size = unsafe {
            let mut subgroup_properties = PhysicalDeviceSubgroupProperties::default();
            let mut physical_device_properties =
                PhysicalDeviceProperties2::default().push_next(&mut subgroup_properties);
            vk.instance.get_physical_device_properties2(
                vk.physical_device,
                &mut physical_device_properties,
            );

            subgroup_properties.subgroup_size
        };

        // Buffer length = the number of dispatches * 2 bytes
        let dispatches_x = source_size[0].div_ceil(32);
        let dispatches_y = source_size[1].div_ceil(32).div_ceil(subgroup_size);
        let buffer_length_bytes = (dispatches_x * dispatches_y) * 2;

        // Setup "read" buffer
        let (read_buffer, read_buffer_memory) = vk
            .create_bound_buffer(
                buffer_length_bytes as u64,
                BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .map_err(|e| Error::Vulkan(e, "creating read buffer"))?;

        // Setup "write" buffer
        let (write_buffer, write_buffer_memory) = vk
            .create_bound_buffer(
                buffer_length_bytes as u64,
                BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
                MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .map_err(|e| Error::Vulkan(e, "creating write buffer"))?;

        // Perform reduction on source writing results to read buffer
        self.source_pass.run(
            vk,
            source,
            source_size,
            read_buffer,
            subgroup_size,
            self.fences[0],
            self.command_buffers[0],
            self.semaphores[0],
        )?;

        // finish reduction over read and write buffers until final result
        let result_buffer = self.buffer_pass.run(
            vk,
            &self.command_buffers,
            &self.fences,
            &self.semaphores,
            read_buffer,
            write_buffer,
            buffer_length_bytes,
            subgroup_size,
        )?;

        // wait for fences
        unsafe {
            vk.device
                .wait_for_fences(&self.fences, true, u64::MAX)
                .map_err(|e| Error::Vulkan(e, "waiting for fences"))?
        }

        // Retrieve maximum from result buffer
        let (staging_buffer, staging_buffer_memory) = vk
            .create_bound_buffer(
                4,
                BufferUsageFlags::TRANSFER_DST,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )
            .map_err(|e| Error::Vulkan(e, "creating staging buffer"))?;

        // copy from result to staging buffer
        vk.record_submit_command_buffer(
            self.command_buffers[0],
            self.fences[0],
            &[],
            &[],
            |device, command_buffer| unsafe {
                let memory_barrier = BufferMemoryBarrier2 {
                    src_stage_mask: PipelineStageFlags2::NONE,
                    src_access_mask: AccessFlags2::NONE,
                    dst_stage_mask: PipelineStageFlags2::TRANSFER,
                    dst_access_mask: AccessFlags2::MEMORY_READ,
                    src_queue_family_index: QUEUE_FAMILY_IGNORED,
                    dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                    buffer: result_buffer,
                    offset: 0,
                    size: 4,
                    ..Default::default()
                };
                let memory_barriers = &[memory_barrier];

                let dependency_info =
                    DependencyInfo::default().buffer_memory_barriers(memory_barriers);

                device.cmd_pipeline_barrier2(command_buffer, &dependency_info);

                // copy
                let buffer_copy = BufferCopy2 {
                    src_offset: 0,
                    dst_offset: 0,
                    size: 4,
                    ..Default::default()
                };
                let buffer_copy_regions = &[buffer_copy];

                let buffer_copy_info = CopyBufferInfo2::default()
                    .src_buffer(result_buffer)
                    .dst_buffer(staging_buffer)
                    .regions(buffer_copy_regions);

                device.cmd_copy_buffer2(command_buffer, &buffer_copy_info);
                Ok(())
            },
        )?;

        unsafe {
            vk.device
                .wait_for_fences(&self.fences, true, u64::MAX)
                .map_err(|e| Error::Vulkan(e, "waiting for fences"))?;
        }

        let maximum = unsafe {
            let memory_ptr = vk
                .device
                .map_memory(staging_buffer_memory, 0, 4, MemoryMapFlags::empty())
                .map_err(|e| Error::Vulkan(e, "mapping staging buffer memory"))?;

            let data = std::slice::from_raw_parts(memory_ptr.cast(), 4);

            let maximum = f16::from_le_bytes([data[0], data[1]]);

            vk.device.unmap_memory(staging_buffer_memory);

            maximum
        };

        unsafe {
            vk.device.destroy_buffer(read_buffer, None);
            vk.device.free_memory(read_buffer_memory, None);

            vk.device.destroy_buffer(write_buffer, None);
            vk.device.free_memory(write_buffer_memory, None);

            vk.device.destroy_buffer(staging_buffer, None);
            vk.device.free_memory(staging_buffer_memory, None);
        }

        Ok(maximum)
    }
}

impl Drop for Maximum {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.buffer_pass.drop(&self.device);
            self.source_pass.drop(&self.device);
            self.fences
                .iter()
                .for_each(|&fence| self.device.destroy_fence(fence, None));
            self.semaphores
                .iter()
                .for_each(|&semaphore| self.device.destroy_semaphore(semaphore, None));
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to read shader:\n{0}")]
    ReadShader(#[source] std::io::Error),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] vulkan_instance::record_submit_command_buffer::Error),
}
