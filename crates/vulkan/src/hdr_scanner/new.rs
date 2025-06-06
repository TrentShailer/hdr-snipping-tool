use core::slice;

use ash::{ext, vk};
use ash_helper::{Context, VkError, VulkanContext, allocate_buffer, try_name, try_name_all};

use crate::{Vulkan, shaders::maximum_reduction};

use super::{HdrScanner, HdrScannerError};

impl<'vulkan> HdrScanner<'vulkan> {
    /// Creates a new HDR Scanner.
    pub fn new(vulkan: &'vulkan Vulkan) -> Result<Self, HdrScannerError> {
        // Descriptor layouts
        let descriptor_layouts = {
            let layouts = unsafe {
                maximum_reduction::set_layouts(
                    vulkan.device(),
                    vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
                )
                .map_err(|e| VkError::new(e, "vkCreateDescriptorSetLayout"))?
            };

            unsafe { try_name_all(vulkan, &layouts, "HdrScanner Descriptor Layout") };

            layouts
        };

        // Pipeline layout
        let pipeline_layout = {
            let create_info =
                vk::PipelineLayoutCreateInfo::default().set_layouts(&descriptor_layouts);

            let layout = unsafe { vulkan.device().create_pipeline_layout(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreatePiplineLayout"))?;

            unsafe { try_name(vulkan, layout, "HdrScanner Pipeline Layout") };

            layout
        };

        // Shader object
        let shader = {
            let create_info = vk::ShaderCreateInfoEXT::default()
                .code(maximum_reduction::BYTES)
                .code_type(vk::ShaderCodeTypeEXT::SPIRV)
                .stage(maximum_reduction::compute_main::STAGE)
                .name(maximum_reduction::compute_main::ENTRY_POINT)
                .set_layouts(&descriptor_layouts);

            let device: &ext::shader_object::Device = unsafe { vulkan.context() };
            let shaders = unsafe { device.create_shaders(slice::from_ref(&create_info), None) }
                .map_err(|(_, e)| VkError::new(e, "vkCreateShadersEXT"))?;

            let shader = shaders[0];
            unsafe { try_name(vulkan, shader, "HdrScanner Compute Shader") };

            shader
        };

        // Buffers
        let (buffer, memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(vulkan.queue_family_index_as_slice())
                .size(4)
                .usage(
                    vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_SRC
                        | vk::BufferUsageFlags::TRANSFER_DST,
                );

            unsafe {
                allocate_buffer(
                    vulkan,
                    &create_info,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    "HDR Scanner",
                )?
            }
        };

        let (staging_buffer, staging_memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .queue_family_indices(vulkan.queue_family_index_as_slice())
                .size(4)
                .usage(vk::BufferUsageFlags::TRANSFER_DST);

            unsafe {
                allocate_buffer(
                    vulkan,
                    &create_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "HDR Scanner Staging",
                )?
            }
        };

        // Command objects
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
                try_name(vulkan, pool, "HdrScanner Command Pool");
                try_name(vulkan, buffer, "HdrScanner Command Buffer");
                try_name(vulkan, semaphore, "HdrScanner Semaphore");
            }

            (pool, buffer, semaphore)
        };

        Ok(Self {
            vulkan,
            descriptor_layouts,
            pipeline_layout,
            shader,
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
