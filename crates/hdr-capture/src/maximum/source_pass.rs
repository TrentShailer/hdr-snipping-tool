use std::sync::Arc;

use ash::vk::{
    Buffer, CommandBuffer, ComputePipelineCreateInfo, DescriptorBufferInfo, DescriptorImageInfo,
    DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet,
    DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateInfo, DescriptorType, Fence, ImageLayout, ImageView, Pipeline,
    PipelineBindPoint, PipelineCache, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineShaderStageCreateInfo, PipelineStageFlags2, Sampler, Semaphore, ShaderModule,
    ShaderStageFlags, WriteDescriptorSet,
};
use tracing::{info_span, instrument};
use vulkan_instance::{VulkanError, VulkanInstance};

use super::Error;

pub struct SourcePass {
    vk: Arc<VulkanInstance>,
    module: ShaderModule,
    layout: PipelineLayout,
    pipeline: Pipeline,
    descriptor_pool: DescriptorPool,
    descriptor_layouts: [DescriptorSetLayout; 1],
    descriptor_sets: Vec<DescriptorSet>,
}

impl SourcePass {
    #[instrument("SourcePass::new", skip_all, err)]
    pub fn new(vk: Arc<VulkanInstance>) -> Result<Self, Error> {
        let (shader_module, shader_entry_name) = unsafe {
            let module =
                vk.create_shader_module(include_bytes!("./shaders/maximum_source_pass.spv"))?;
            let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");

            (module, shader_entry_name)
        };

        let shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(shader_entry_name);

        let descriptor_pool = unsafe {
            let descriptor_sizes = [
                DescriptorPoolSize {
                    ty: DescriptorType::STORAGE_IMAGE,
                    descriptor_count: 1,
                },
                DescriptorPoolSize {
                    ty: DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
            ];
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
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
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

        let (compute_pipeline, pipeline_layout) = unsafe {
            let layout_create_info =
                PipelineLayoutCreateInfo::default().set_layouts(&descriptor_layouts);

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
            vk,
            module: shader_module,
            layout: pipeline_layout,
            pipeline: compute_pipeline,
            descriptor_pool,
            descriptor_layouts,
            descriptor_sets,
        })
    }

    #[instrument("SourcePass::run", skip_all, err)]
    pub fn run(
        &self,
        source: ImageView,
        source_size: [u32; 2],
        output_buffer: Buffer,
        subgroup_size: u32,
        submission_resources: ((CommandBuffer, Fence), Semaphore),
    ) -> Result<(), Error> {
        // Update descriptor sets
        unsafe {
            let input_image_descriptor = [DescriptorImageInfo {
                sampler: Sampler::null(),
                image_view: source,
                image_layout: ImageLayout::GENERAL,
            }];

            let output_buffer_descriptor = [DescriptorBufferInfo {
                buffer: output_buffer,
                offset: 0,
                range: ash::vk::WHOLE_SIZE,
            }];

            let write_descriptor_sets = [
                WriteDescriptorSet::default()
                    .dst_set(self.descriptor_sets[0])
                    .dst_binding(0)
                    .descriptor_type(DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .image_info(&input_image_descriptor),
                WriteDescriptorSet::default()
                    .dst_set(self.descriptor_sets[0])
                    .dst_binding(1)
                    .descriptor_type(DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .buffer_info(&output_buffer_descriptor),
            ];

            self.vk
                .device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        // Dispatch
        let workgroup_x = source_size[0].div_ceil(32);
        let workgroup_y = source_size[1].div_ceil(32).div_ceil(subgroup_size);

        let signal_semaphores = [(submission_resources.1, PipelineStageFlags2::BOTTOM_OF_PIPE)];
        self.vk.record_submit_command_buffer(
            submission_resources.0,
            &[],
            &signal_semaphores,
            |device, command_buffer| unsafe {
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    PipelineBindPoint::COMPUTE,
                    self.layout,
                    0,
                    &self.descriptor_sets,
                    &[],
                );

                device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::COMPUTE, self.pipeline);

                device.cmd_dispatch(command_buffer, workgroup_x, workgroup_y, 1);

                Ok(())
            },
        )?;

        Ok(())
    }
}

impl Drop for SourcePass {
    fn drop(&mut self) {
        let _span = info_span!("SourcePass::Drop").entered();
        unsafe {
            self.vk.device.device_wait_idle().unwrap();
            self.vk
                .device
                .destroy_descriptor_set_layout(self.descriptor_layouts[0], None);
            self.vk
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.vk.device.destroy_pipeline(self.pipeline, None);
            self.vk.device.destroy_pipeline_layout(self.layout, None);
            self.vk.device.destroy_shader_module(self.module, None);
        }
    }
}
