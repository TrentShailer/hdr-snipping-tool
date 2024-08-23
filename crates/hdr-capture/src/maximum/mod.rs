mod buffer_pass;
mod source_pass;

use ash::{
    vk::{
        AccessFlags2, BufferCopy2, BufferUsageFlags, CommandBuffer, CopyBufferInfo2,
        DependencyInfo, Fence, MemoryPropertyFlags, PhysicalDeviceProperties2,
        PhysicalDeviceSubgroupProperties, PipelineStageFlags2, Semaphore, SemaphoreCreateInfo,
    },
    Device,
};
use buffer_pass::BufferPass;
use half::f16;
use source_pass::SourcePass;
use thiserror::Error;
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::hdr_capture::HdrCapture;

pub(crate) const MAXIMUM_SUBMISSIONS: usize = 3;

pub struct Maximum<'d> {
    device: &'d Device,

    command_buffers: Box<[(CommandBuffer, Fence)]>,
    semaphores: Vec<Semaphore>,

    source_pass: SourcePass<'d>,
    buffer_pass: BufferPass<'d>,
}

impl<'d> Maximum<'d> {
    #[instrument("Maximum::new", skip_all, err)]
    pub fn new(vk: &'d VulkanInstance) -> Result<Self, Error> {
        // create command buffers
        let command_buffers = vk.allocate_command_buffers(MAXIMUM_SUBMISSIONS as u32)?;

        // create semaphores
        let semaphore_create_info = SemaphoreCreateInfo::default();
        let semaphores: Vec<Semaphore> = (0..MAXIMUM_SUBMISSIONS)
            .map(|_| unsafe {
                vk.device
                    .create_semaphore(&semaphore_create_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating semaphore"))
            })
            .collect::<Result<_, VulkanError>>()?;

        let source_pass = SourcePass::new(vk)?;
        let buffer_pass = BufferPass::new(vk)?;

        Ok(Self {
            device: &vk.device,
            command_buffers,
            semaphores,
            source_pass,
            buffer_pass,
        })
    }

    #[instrument("Maximum::find_maximum", skip_all, err)]
    pub fn find_maximum(
        &self,
        vk: &VulkanInstance,
        hdr_capture: &HdrCapture,
    ) -> Result<f16, Error> {
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
        let dispatches_x = hdr_capture.size[0].div_ceil(32);
        let dispatches_y = hdr_capture.size[1].div_ceil(32).div_ceil(subgroup_size);
        let buffer_length_bytes = (dispatches_x * dispatches_y) * 2;

        // Setup "read" buffer
        let (read_buffer, read_buffer_memory) = vk.create_bound_buffer(
            buffer_length_bytes as u64,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Setup "write" buffer
        let (write_buffer, write_buffer_memory) = vk.create_bound_buffer(
            buffer_length_bytes as u64,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Perform reduction on source writing results to read buffer
        self.source_pass.run(
            vk,
            hdr_capture.image_view,
            hdr_capture.size,
            read_buffer,
            subgroup_size,
            (self.command_buffers[0], self.semaphores[0]),
        )?;

        // finish reduction over read and write buffers until final result
        let result_buffer = self.buffer_pass.run(
            vk,
            self,
            read_buffer,
            write_buffer,
            buffer_length_bytes,
            subgroup_size,
        )?;

        // wait for fences
        let fences = [
            self.command_buffers[0].1,
            self.command_buffers[1].1,
            self.command_buffers[2].1,
        ];
        unsafe {
            vk.device
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(|e| VulkanError::VkResult(e, "waiting for fences"))?
        }

        // Retrieve maximum from result buffer
        let (staging_buffer, staging_buffer_memory) = vk.create_bound_buffer(
            4,
            BufferUsageFlags::TRANSFER_DST,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // copy from result to staging buffer
        vk.record_submit_command_buffer(
            self.command_buffers[0],
            &[],
            &[],
            |device, command_buffer| unsafe {
                let memory_barrier = VulkanInstance::buffer_memory_barrier()
                    .dst_stage_mask(PipelineStageFlags2::TRANSFER)
                    .dst_access_mask(AccessFlags2::MEMORY_READ)
                    .size(4)
                    .buffer(result_buffer);
                let memory_barriers = &[memory_barrier];

                let dependency_info =
                    DependencyInfo::default().buffer_memory_barriers(memory_barriers);

                device.cmd_pipeline_barrier2(command_buffer, &dependency_info);

                // copy
                let buffer_copy = BufferCopy2::default().size(3);
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
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(|e| VulkanError::VkResult(e, "waiting for fences"))?;
        }

        let maximums: &[f16] = unsafe { vk.read_from_memory(staging_buffer_memory, 0, 4)? };
        let maximum = maximums[0];

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

impl<'d> Drop for Maximum<'d> {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.command_buffers
                .iter()
                .for_each(|&(_, fence)| self.device.destroy_fence(fence, None));
            self.semaphores
                .iter()
                .for_each(|&semaphore| self.device.destroy_semaphore(semaphore, None));
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Vulkan(#[from] VulkanError),
}
