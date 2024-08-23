use ash::{
    vk::{
        ComputePipelineCreateInfo, DependencyInfo, DescriptorImageInfo, DescriptorPool,
        DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
        DescriptorType, DeviceMemory, Extent2D, Format, Image, ImageAspectFlags, ImageCreateInfo,
        ImageLayout, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
        ImageViewCreateInfo, ImageViewType, MemoryAllocateInfo, MemoryPropertyFlags, Pipeline,
        PipelineBindPoint, PipelineCache, PipelineLayout, PipelineLayoutCreateInfo,
        PipelineShaderStageCreateInfo, PipelineStageFlags2, PushConstantRange, SampleCountFlags,
        Sampler, ShaderModule, ShaderStageFlags, SharingMode, WriteDescriptorSet,
    },
    Device,
};
use thiserror::Error;
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::HdrCapture;

pub struct Tonemap<'d> {
    device: &'d Device,
    module: ShaderModule,
    layout: PipelineLayout,
    pipeline: Pipeline,
    descriptor_pool: DescriptorPool,
    descriptor_layouts: [DescriptorSetLayout; 1],
    descriptor_sets: Vec<DescriptorSet>,
}

impl<'d> Tonemap<'d> {
    #[instrument("Tonemap::new", skip_all, err)]
    pub fn new(vk: &'d VulkanInstance) -> Result<Self, Error> {
        let (shader_module, shader_entry_name) = unsafe {
            let module = vk.create_shader_module(include_bytes!("./shaders/scRGB_to_sRGB.spv"))?;
            let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");

            (module, shader_entry_name)
        };

        let shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(shader_entry_name);

        let descriptor_pool = unsafe {
            let descriptor_sizes = [DescriptorPoolSize {
                ty: DescriptorType::STORAGE_IMAGE,
                descriptor_count: 2,
            }];
            let descriptor_pool_info = DescriptorPoolCreateInfo::default()
                .pool_sizes(&descriptor_sizes)
                .max_sets(1);

            vk.device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating descriptor pool"))?
        };

        let descriptor_layouts = unsafe {
            let descriptor_layout_bindings = [
                DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::STORAGE_IMAGE,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::COMPUTE,
                    ..Default::default()
                },
                DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: DescriptorType::STORAGE_IMAGE,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::COMPUTE,
                    ..Default::default()
                },
            ];

            let descriptor_info =
                DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_layout_bindings);

            let descriptor_layout = vk
                .device
                .create_descriptor_set_layout(&descriptor_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating descriptor set layout"))?;

            [descriptor_layout]
        };

        let descriptor_sets = unsafe {
            let descriptor_allocate_info = DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_layouts);

            vk.device
                .allocate_descriptor_sets(&descriptor_allocate_info)
                .map_err(|e| VulkanError::VkResult(e, "allocating descriptor sets"))?
        };

        let push_constant_ranges = [PushConstantRange {
            stage_flags: ShaderStageFlags::COMPUTE,
            offset: 0,
            size: 4,
        }];

        let (compute_pipeline, pipeline_layout) = unsafe {
            let layout_create_info = PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_layouts)
                .push_constant_ranges(&push_constant_ranges);

            let pipeline_layout = vk
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating pipeline layout"))?;

            let compute_pipeline_create_info = ComputePipelineCreateInfo::default()
                .stage(shader_stage_create_info)
                .layout(pipeline_layout);

            let compute_pipeline = vk
                .device
                .create_compute_pipelines(
                    PipelineCache::null(),
                    &[compute_pipeline_create_info],
                    None,
                )
                .map_err(|(_, e)| VulkanError::VkResult(e, "creating compute pipline"))?;

            (compute_pipeline[0], pipeline_layout)
        };

        Ok(Self {
            device: &vk.device,
            layout: pipeline_layout,
            pipeline: compute_pipeline,
            descriptor_sets,
            descriptor_layouts,
            descriptor_pool,
            module: shader_module,
        })
    }

    #[instrument("Tonemap::tonemap", skip_all, err)]
    pub fn tonemap(
        &self,
        vk: &VulkanInstance,
        hdr_capture: &HdrCapture,
    ) -> Result<(Image, DeviceMemory, ImageView), Error> {
        let (image, image_memory, image_view) = unsafe {
            let image = {
                let image_extent = Extent2D {
                    width: hdr_capture.size[0],
                    height: hdr_capture.size[1],
                };

                let image_create_info = ImageCreateInfo::default()
                    .image_type(ImageType::TYPE_2D)
                    .format(Format::R8G8B8A8_UNORM)
                    .extent(image_extent.into())
                    .mip_levels(1)
                    .array_layers(1)
                    .samples(SampleCountFlags::TYPE_1)
                    .tiling(ImageTiling::OPTIMAL)
                    .usage(ImageUsageFlags::STORAGE)
                    .sharing_mode(SharingMode::EXCLUSIVE)
                    .initial_layout(ImageLayout::UNDEFINED);

                vk.device
                    .create_image(&image_create_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating image"))?
            };

            // Create and import memory.
            let memory = {
                let memory_requirement = vk.device.get_image_memory_requirements(image);

                let memory_index = vk
                    .find_memorytype_index(&memory_requirement, MemoryPropertyFlags::DEVICE_LOCAL)
                    .ok_or(VulkanError::NoSuitableMemoryType)?;

                let allocate_info = MemoryAllocateInfo::default()
                    .allocation_size(memory_requirement.size)
                    .memory_type_index(memory_index);

                let device_memory = vk.device.allocate_memory(&allocate_info, None).unwrap();

                vk.device
                    .bind_image_memory(image, device_memory, 0)
                    .unwrap();

                device_memory
            };

            // Create the image view
            let image_view = {
                let image_view_create_info = ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(ImageViewType::TYPE_2D)
                    .format(Format::R8G8B8A8_UNORM)
                    .subresource_range(
                        ImageSubresourceRange::default()
                            .aspect_mask(ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .layer_count(1),
                    );

                vk.device
                    .create_image_view(&image_view_create_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating image view"))?
            };

            (image, memory, image_view)
        };

        unsafe {
            let input_image_descriptor = DescriptorImageInfo {
                sampler: Sampler::null(),
                image_view: hdr_capture.image_view,
                image_layout: ImageLayout::GENERAL,
            };

            let output_image_descriptor = DescriptorImageInfo {
                sampler: Sampler::null(),
                image_view,
                image_layout: ImageLayout::GENERAL,
            };

            let write_descriptor_sets = [
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_IMAGE,
                    p_image_info: &input_image_descriptor,
                    ..Default::default()
                },
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_IMAGE,
                    p_image_info: &output_image_descriptor,
                    ..Default::default()
                },
            ];

            vk.device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        // Shader tonemaps a 32x32 area each dispatch
        let workgroup_x = hdr_capture.size[0].div_ceil(32);
        let workgroup_y = hdr_capture.size[1].div_ceil(32);

        let push_constants = hdr_capture.whitepoint.to_le_bytes();

        vk.record_submit_command_buffer(
            vk.command_buffer,
            &[],
            &[],
            |device, command_buffer| unsafe {
                let memory_barriers = [VulkanInstance::image_memory_barrier()
                    .old_layout(ImageLayout::UNDEFINED)
                    .new_layout(ImageLayout::GENERAL)
                    .dst_stage_mask(PipelineStageFlags2::COMPUTE_SHADER)
                    .image(image)];

                let dependency_info =
                    DependencyInfo::default().image_memory_barriers(&memory_barriers);

                device.cmd_pipeline_barrier2(command_buffer, &dependency_info);

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    PipelineBindPoint::COMPUTE,
                    self.layout,
                    0,
                    &self.descriptor_sets,
                    &[],
                );

                device.cmd_push_constants(
                    command_buffer,
                    self.layout,
                    ShaderStageFlags::COMPUTE,
                    0,
                    &push_constants,
                );

                device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::COMPUTE, self.pipeline);

                device.cmd_dispatch(command_buffer, workgroup_x, workgroup_y, 1);
                Ok(())
            },
        )?;

        unsafe {
            vk.device
                .wait_for_fences(&[vk.command_buffer.1], true, u64::MAX)
        }
        .map_err(|e| VulkanError::VkResult(e, "waiting for fence"))?;

        Ok((image, image_memory, image_view))
    }
}

impl<'d> Drop for Tonemap<'d> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.descriptor_layouts[0], None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_shader_module(self.module, None);
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Vulkan(#[from] VulkanError),
}
