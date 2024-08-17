use std::io::Cursor;

use ash::{
    util::read_spv,
    vk::{
        AccessFlags2, Buffer, BufferCopy2, BufferMemoryBarrier2, BufferUsageFlags,
        ComputePipelineCreateInfo, CopyBufferInfo2, DependencyInfo, DescriptorBufferInfo,
        DescriptorPoolCreateInfo, DescriptorPoolSize, DescriptorSetAllocateInfo,
        DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, MemoryMapFlags,
        MemoryPropertyFlags, PipelineBindPoint, PipelineCache, PipelineLayoutCreateInfo,
        PipelineShaderStageCreateInfo, PipelineStageFlags2, PushConstantRange,
        ShaderModuleCreateInfo, ShaderStageFlags, WriteDescriptorSet, QUEUE_FAMILY_IGNORED,
    },
};
use half::f16;
use tracing::info_span;
use vulkan_instance::{CommandBufferUsage, VulkanInstance};

use super::Error;

/// Performs a gpu reduction over two buffers.
pub(crate) fn buffer_reduction(
    vk: &VulkanInstance,
    read_buffer: Buffer,
    write_buffer: Buffer,
    byte_count: u32,
    subgroup_size: u32,
) -> Result<f16, Error> {
    let _span = info_span!("buffer_reduction").entered();

    // 1024 threads * sugroup_size
    // This is how much the input gets reduced by on a single pass
    let compute_blocksize = 1024 * subgroup_size;

    let (shader_module, shader_entry_name) = unsafe {
        let mut shader_file =
            Cursor::new(&include_bytes!("../shaders/maximum_buffer_pass.spv")[..]);
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
        let descriptor_sizes = [DescriptorPoolSize {
            ty: DescriptorType::STORAGE_BUFFER,
            descriptor_count: 4,
        }];
        let descriptor_pool_info = DescriptorPoolCreateInfo::default()
            .pool_sizes(&descriptor_sizes)
            .max_sets(2);

        vk.device
            .create_descriptor_pool(&descriptor_pool_info, None)
            .map_err(|e| Error::Vulkan(e, "creating descriptor pool"))?
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
            .map_err(|e| Error::Vulkan(e, "creating descriptor set layout"))?;

        [descriptor_layout, descriptor_layout]
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
                dst_set: descriptor_sets[0],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &read_buffer_descriptor,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &write_buffer_descriptor,
                ..Default::default()
            },
            // inverse
            WriteDescriptorSet {
                dst_set: descriptor_sets[1],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &read_buffer_descriptor,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: descriptor_sets[1],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &write_buffer_descriptor,
                ..Default::default()
            },
        ];

        vk.device
            .update_descriptor_sets(&write_descriptor_sets, &[]);
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
            .map_err(|e| Error::Vulkan(e, "creating pipeline layout"))?;

        let compute_pipeline_create_info = ComputePipelineCreateInfo::default()
            .stage(shader_stage_create_info)
            .layout(pipeline_layout);

        let compute_pipeline = vk
            .device
            .create_compute_pipelines(PipelineCache::null(), &[compute_pipeline_create_info], None)
            .map_err(|(_, e)| Error::Vulkan(e, "creating compute pipline"))?;

        (compute_pipeline[0], pipeline_layout)
    };

    // Dispatch

    let mut input_length = byte_count / 2;
    let mut output_length = (byte_count / 2).div_ceil(compute_blocksize);
    let mut use_write_read_ds = true;

    while input_length > 1 {
        let _span = info_span!("pass").entered();
        let workgroup_count = output_length;

        use_write_read_ds = !use_write_read_ds;

        // Perform reduction pass
        {
            let descriptor_set = if use_write_read_ds {
                descriptor_sets[1]
            } else {
                descriptor_sets[0]
            };

            vk.record_submit_command_buffer(
                CommandBufferUsage::Tonemap,
                &[],
                &[],
                |device, command_buffer| unsafe {
                    device.cmd_bind_descriptor_sets(
                        command_buffer,
                        PipelineBindPoint::COMPUTE,
                        pipeline_layout,
                        0,
                        &[descriptor_set],
                        &[],
                    );

                    device.cmd_bind_pipeline(
                        command_buffer,
                        PipelineBindPoint::COMPUTE,
                        compute_pipeline,
                    );

                    device.cmd_push_constants(
                        command_buffer,
                        pipeline_layout,
                        ShaderStageFlags::COMPUTE,
                        0,
                        &input_length.to_le_bytes(),
                    );

                    device.cmd_dispatch(command_buffer, workgroup_count, 1, 1);
                },
            )?;

            unsafe {
                vk.device.wait_for_fences(
                    &[*vk.fences.get(&CommandBufferUsage::Tonemap).unwrap()],
                    true,
                    u64::MAX,
                )
            }
            .map_err(|e| Error::Vulkan(e, "waiting for fence"))?;
        }

        // calculate updated input and output lengths
        input_length = output_length;
        output_length = input_length.div_ceil(compute_blocksize);
    }

    // Find what buffer has the final result in it.
    let result_buffer = if use_write_read_ds {
        read_buffer
    } else {
        write_buffer
    };

    // Setup CPU staging buffer for GPU to write data to.
    let (staging_buffer, staging_buffer_memory) = vk
        .create_bound_buffer(
            4,
            BufferUsageFlags::TRANSFER_DST,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
        .map_err(|e| Error::Vulkan(e, "creating staging buffer"))?;

    // copy from result to staging buffer
    vk.record_submit_command_buffer(
        CommandBufferUsage::Setup,
        &[],
        &[],
        |device, command_buffer| {
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

            let dependency_info = DependencyInfo::default().buffer_memory_barriers(memory_barriers);

            unsafe { device.cmd_pipeline_barrier2(command_buffer, &dependency_info) }

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

            unsafe { device.cmd_copy_buffer2(command_buffer, &buffer_copy_info) }
        },
    )
    .unwrap();

    unsafe {
        vk.device.wait_for_fences(
            &[*vk.fences.get(&CommandBufferUsage::Setup).unwrap()],
            true,
            u64::MAX,
        )
    }
    .unwrap();

    //
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

    // cleanup resources
    unsafe {
        vk.device.destroy_buffer(staging_buffer, None);
        vk.device.free_memory(staging_buffer_memory, None);

        vk.device.destroy_pipeline_layout(pipeline_layout, None);
        vk.device.destroy_pipeline(compute_pipeline, None);

        vk.device
            .destroy_descriptor_set_layout(descriptor_layouts[0], None);
        vk.device.destroy_descriptor_pool(descriptor_pool, None);

        vk.device.destroy_shader_module(shader_module, None);
    }

    Ok(maximum)
}
