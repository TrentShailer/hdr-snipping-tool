use std::io::Cursor;

use ash::{
    util::read_spv,
    vk::{
        Buffer, CommandBuffer, ComputePipelineCreateInfo, DescriptorBufferInfo,
        DescriptorImageInfo, DescriptorPoolCreateInfo, DescriptorPoolSize,
        DescriptorSetAllocateInfo, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
        DescriptorType, ImageLayout, ImageView, PipelineBindPoint, PipelineCache,
        PipelineLayoutCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags2, Sampler,
        Semaphore, ShaderModuleCreateInfo, ShaderStageFlags, WriteDescriptorSet,
    },
    Device,
};
use tracing::info_span;
use vulkan_instance::{CommandBufferUsage, VulkanInstance};

use super::{Error, Maximum};

pub struct SourcePass {
    module: ShaderModule,
    layout: PipelineLayout,
    pipeline: Pipeline,
    descriptor_pool: DescriptorPool,
    descriptor_layouts: [DescriptorSetLayout; 1],
    descriptor_sets: Vec<DescriptorSet>,
}

impl SourcePass {
    pub fn new(vk: &VulkanInstance) -> Result<Self, Error> {
        let _span = info_span!("SourcePass::new").entered();

        let (shader_module, shader_entry_name) = unsafe {
            let mut shader_file =
                Cursor::new(&include_bytes!("../shaders/maximum_source_pass.spv")[..]);
            let shader_code = read_spv(&mut shader_file).map_err(Error::ReadShader)?;
            let shader_info = ShaderModuleCreateInfo::default().code(&shader_code);
            let shader_module = vk
                .device
                .create_shader_module(&shader_info, None)
                .map_err(|e| Error::Vulkan(e, "creating shader module"))?;
            let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");

            (shader_module, shader_entry_name)
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
                .map_err(|e| Error::Vulkan(e, "creating descriptor pool"))?
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
                .map_err(|e| Error::Vulkan(e, "creating descriptor set layout"))?;

            [descriptor_layout]
        };

        let descriptor_sets = unsafe {
            let descriptor_allocate_info = DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_layouts);

            let descriptor_sets = vk
                .device
                .allocate_descriptor_sets(&descriptor_allocate_info)
                .map_err(|e| Error::Vulkan(e, "allocating descriptor sets"))?;

            descriptor_sets
        };

        let (compute_pipeline, pipeline_layout) = unsafe {
            let layout_create_info =
                PipelineLayoutCreateInfo::default().set_layouts(&descriptor_layouts);

            let pipeline_layout = vk
                .device
                .create_pipeline_layout(&layout_create_info, None)
                .map_err(|e| Error::Vulkan(e, "creating pipeline layout"))?;

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
                .map_err(|(_, e)| Error::Vulkan(e, "creating compute pipline"))?;

            (compute_pipeline[0], pipeline_layout)
        };

        Ok(Self {
            module: shader_module,
            layout: pipeline_layout,
            pipeline: compute_pipeline,
            descriptor_pool,
            descriptor_layouts,
            descriptor_sets,
        })
    }

    pub fn run(
        &self,
        vk: &VulkanInstance,
        source: ImageView,
        source_size: [u32; 2],
        output_buffer: Buffer,
        subgroup_size: u32,
        //
        fence: Fence,
        command_buffer: CommandBuffer,
        signal_semaphore: Semaphore,
    ) -> Result<(), Error> {
        let _span = info_span!("SourcePass::run").entered();

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

            vk.device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        // Dispatch
        let workgroup_x = source_size[0].div_ceil(32);
        let workgroup_y = source_size[1].div_ceil(32).div_ceil(subgroup_size);

        let signal_semaphores = [(signal_semaphore, PipelineStageFlags2::BOTTOM_OF_PIPE)];
        vk.record_submit_command_buffer(
            command_buffer,
            fence,
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
        );

        Ok(())
    }

    pub fn drop(&self, device: &Device) {
        unsafe {
            device.destroy_descriptor_set_layout(descriptor_layouts[0], None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_shader_module(self.module, None);
        }
    }
}
