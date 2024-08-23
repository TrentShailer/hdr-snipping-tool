use std::sync::Arc;

use ash::vk::{
    Buffer, CommandBuffer, ComputePipelineCreateInfo, DescriptorBufferInfo, DescriptorPool,
    DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
    DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType,
    Fence, Pipeline, PipelineBindPoint, PipelineCache, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineShaderStageCreateInfo, PipelineStageFlags2, PushConstantRange, Semaphore, ShaderModule,
    ShaderStageFlags, WriteDescriptorSet,
};

use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use super::{Error, Maximum, MAXIMUM_SUBMISSIONS};

pub struct BufferPass {
    vk: Arc<VulkanInstance>,

    module: ShaderModule,
    layout: PipelineLayout,
    pipeline: Pipeline,
    descriptor_pool: DescriptorPool,
    descriptor_layouts: [DescriptorSetLayout; 2],
    descriptor_sets: Vec<DescriptorSet>,
}

impl BufferPass {
    #[instrument("BufferPass::new", skip_all, err)]
    pub fn new(vk: Arc<VulkanInstance>) -> Result<Self, Error> {
        let (shader_module, shader_entry_name) = unsafe {
            let module =
                vk.create_shader_module(include_bytes!("./shaders/maximum_buffer_pass.spv"))?;
            let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");

            (module, shader_entry_name)
        };

        let shader_stage_create_info = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(shader_entry_name);

        let descriptor_pool = unsafe {
            let descriptor_sizes = [DescriptorPoolSize {
                ty: DescriptorType::STORAGE_BUFFER,
                descriptor_count: 4,
            }];
            let descriptor_pool_info = DescriptorPoolCreateInfo::default()
                .pool_sizes(&descriptor_sizes)
                .max_sets(2);

            vk.device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .map_err(|e| VulkanError::VkResult(e, "creating descriptor pool"))?
        };

        let descriptor_layouts = unsafe {
            let descriptor_layout_bindings = [
                DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
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

            [descriptor_layout, descriptor_layout]
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
            let push_constant_ranges = [PushConstantRange {
                stage_flags: ShaderStageFlags::COMPUTE,
                offset: 0,
                size: 32,
            }];

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
            vk,
            module: shader_module,
            layout: pipeline_layout,
            pipeline: compute_pipeline,
            descriptor_pool,
            descriptor_layouts,
            descriptor_sets,
        })
    }

    #[instrument("BufferPass::run", skip_all, err)]
    pub fn run(
        &self,
        maximum_obj: &Maximum,

        read_buffer: Buffer,
        write_buffer: Buffer,
        byte_count: u32,
        subgroup_size: u32,
    ) -> Result<Buffer, Error> {
        let command_buffers: &[(CommandBuffer, Fence)] = &maximum_obj.command_buffers;
        let semaphores: &[Semaphore] = &maximum_obj.semaphores;

        // Update descriptor sets
        unsafe {
            let read_buffer_descriptor = DescriptorBufferInfo {
                buffer: read_buffer,
                offset: 0,
                range: ash::vk::WHOLE_SIZE,
            };
            let write_buffer_descriptor = DescriptorBufferInfo {
                buffer: write_buffer,
                offset: 0,
                range: ash::vk::WHOLE_SIZE,
            };

            let write_descriptor_sets = [
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &read_buffer_descriptor,
                    ..Default::default()
                },
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[0],
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &write_buffer_descriptor,
                    ..Default::default()
                },
                // inverse
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[1],
                    dst_binding: 1,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &read_buffer_descriptor,
                    ..Default::default()
                },
                WriteDescriptorSet {
                    dst_set: self.descriptor_sets[1],
                    dst_binding: 0,
                    descriptor_count: 1,
                    descriptor_type: DescriptorType::STORAGE_BUFFER,
                    p_buffer_info: &write_buffer_descriptor,
                    ..Default::default()
                },
            ];

            self.vk
                .device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        // 1024 threads * sugroup_size
        // This is how much the input gets reduced by on a single pass
        let compute_blocksize = 1024 * subgroup_size;

        // Dispatches
        let mut input_length = byte_count / 2;
        let mut output_length = (byte_count / 2).div_ceil(compute_blocksize);
        let mut use_write_read_ds = true;
        let mut submission_index = 1;

        while input_length > 1 {
            let workgroup_count = output_length;
            use_write_read_ds = !use_write_read_ds;

            // Perform reduction pass
            {
                let descriptor_set = if use_write_read_ds {
                    self.descriptor_sets[1]
                } else {
                    self.descriptor_sets[0]
                };

                let command_buffer = command_buffers[submission_index];

                let will_have_following_submission = output_length > 1;
                let maybe_signal_semaphores = [(
                    semaphores[submission_index],
                    PipelineStageFlags2::BOTTOM_OF_PIPE,
                )];
                let signal_semaphores: &[(Semaphore, PipelineStageFlags2)] =
                    if will_have_following_submission {
                        &maybe_signal_semaphores
                    } else {
                        &[]
                    };

                let wait_semaphore_index = if submission_index == 0 {
                    MAXIMUM_SUBMISSIONS - 1
                } else {
                    submission_index - 1
                };
                let wait_semaphores = [(
                    semaphores[wait_semaphore_index],
                    PipelineStageFlags2::COMPUTE_SHADER,
                )];

                self.vk.record_submit_command_buffer(
                    command_buffer,
                    &wait_semaphores,
                    signal_semaphores,
                    |device, command_buffer| unsafe {
                        device.cmd_bind_descriptor_sets(
                            command_buffer,
                            PipelineBindPoint::COMPUTE,
                            self.layout,
                            0,
                            &[descriptor_set],
                            &[],
                        );

                        device.cmd_bind_pipeline(
                            command_buffer,
                            PipelineBindPoint::COMPUTE,
                            self.pipeline,
                        );

                        device.cmd_push_constants(
                            command_buffer,
                            self.layout,
                            ShaderStageFlags::COMPUTE,
                            0,
                            &input_length.to_le_bytes(),
                        );

                        device.cmd_dispatch(command_buffer, workgroup_count, 1, 1);
                        Ok(())
                    },
                )?;
            }

            // calculate updated input and output lengths
            input_length = output_length;
            output_length = input_length.div_ceil(compute_blocksize);

            submission_index += 1;
            if submission_index == MAXIMUM_SUBMISSIONS {
                submission_index = 0;
            }
        }

        // find result buffer
        let result_buffer = if use_write_read_ds {
            read_buffer
        } else {
            write_buffer
        };

        Ok(result_buffer)
    }
}

impl Drop for BufferPass {
    fn drop(&mut self) {
        unsafe {
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
