use core::slice;

use alloc::sync::Arc;
use ash::vk;
use ash_helper::{
    VkError, VulkanContext, allocate_buffer, create_shader_module_from_spv, try_name,
};

use crate::Vulkan;

use super::{HdrScanner, HdrScannerError};

impl HdrScanner {
    /// Creates a new HDR Scanner.
    pub unsafe fn new(vulkan: Arc<Vulkan>) -> Result<Self, HdrScannerError> {
        // Create descriptor layout
        let descriptor_layout = {
            let bindings = [
                // Image
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                // Buffer
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
            ];

            let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
                .bindings(&bindings)
                .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR);

            let layout = unsafe {
                vulkan
                    .device()
                    .create_descriptor_set_layout(&layout_info, None)
            }
            .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?;

            unsafe { try_name(vulkan.as_ref(), layout, "HDR Scanner Descriptor Layout") };

            layout
        };

        // Create Pipeline Layout
        let layout = {
            let layout_info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(slice::from_ref(&descriptor_layout));

            let layout = unsafe { vulkan.device().create_pipeline_layout(&layout_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePipelineLayout"))?;

            unsafe { try_name(vulkan.as_ref(), layout, "HDR Scanner Layout") };

            layout
        };

        // Create shader module
        let shader = {
            let shader = unsafe {
                create_shader_module_from_spv(
                    vulkan.as_ref(),
                    include_bytes!("../_shaders/spv/maximum_reduction.spv"),
                )?
            };

            unsafe { try_name(vulkan.as_ref(), shader, "HDR Scanner Shader") };

            shader
        };

        // Create pipeline
        let pipeline = {
            let stage_info = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(shader)
                .name(c"main");

            let create_info = vk::ComputePipelineCreateInfo::default()
                .stage(stage_info)
                .layout(layout);

            let pipeline = unsafe {
                vulkan
                    .device()
                    .create_compute_pipelines(
                        vk::PipelineCache::null(),
                        slice::from_ref(&create_info),
                        None,
                    )
                    .map_err(|e| VkError::new(e.1, "vkCreateComputePipelines"))?[0]
            };

            unsafe { try_name(vulkan.as_ref(), layout, "HDR Scanner Pipeline") };

            pipeline
        };

        let (buffer, memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(vulkan.queue_family_index_as_ref()))
                .size(4)
                .usage(
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_SRC
                        | vk::BufferUsageFlags::TRANSFER_DST,
                );

            unsafe {
                allocate_buffer(
                    vulkan.as_ref(),
                    &create_info,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    "HDR Scanner",
                )?
            }
        };

        let (staging_buffer, staging_memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(slice::from_ref(vulkan.queue_family_index_as_ref()))
                .size(4)
                .usage(vk::BufferUsageFlags::TRANSFER_DST);

            unsafe {
                allocate_buffer(
                    vulkan.as_ref(),
                    &create_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "HDR Scanner Staging",
                )?
            }
        };

        let (command_pool, command_buffer, semaphore) = {
            let pool = {
                let pool_create_info = vk::CommandPoolCreateInfo::default()
                    .queue_family_index(vulkan.queue_family_index());

                unsafe { vulkan.device().create_command_pool(&pool_create_info, None) }
                    .map_err(|e| VkError::new(e, "vkCreateCommandPool"))?
            };

            let buffer = {
                let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);

                unsafe {
                    vulkan
                        .device()
                        .allocate_command_buffers(&command_buffer_info)
                }
                .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0]
            };

            let semaphore = {
                let mut type_info = vk::SemaphoreTypeCreateInfo::default()
                    .initial_value(0)
                    .semaphore_type(vk::SemaphoreType::TIMELINE);
                let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut type_info);

                unsafe { vulkan.device().create_semaphore(&create_info, None) }
                    .map_err(|e| VkError::new(e, "vkCreateSemaphore"))?
            };

            // Debug: Name the objects.
            unsafe {
                try_name(vulkan.as_ref(), pool, "HDR Scanner Pool");
                try_name(vulkan.as_ref(), buffer, "HDR Scanner Command Buffer");
                try_name(vulkan.as_ref(), semaphore, "HDR Scanner Semaphore");
            }

            (pool, buffer, semaphore)
        };

        Ok(Self {
            vulkan,

            descriptor_layout,
            layout,
            shader,
            pipeline,

            semaphore,
            semaphore_value: 0,
            command_pool,
            command_buffer,

            buffer,
            memory,

            staging_buffer,
            staging_memory,
        })
    }
}
