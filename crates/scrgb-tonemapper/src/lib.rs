pub mod maximum;
pub mod tonemap_output;

use std::{
    fmt::Debug,
    io::{self, Cursor},
};

use ash::{
    util::read_spv,
    vk::{
        self, ComputePipelineCreateInfo, DescriptorImageInfo, DescriptorPoolCreateInfo,
        DescriptorPoolSize, DescriptorSetAllocateInfo, DescriptorSetLayoutBinding,
        DescriptorSetLayoutCreateInfo, DescriptorType, ImageLayout, ImageView, PipelineBindPoint,
        PipelineCache, PipelineLayoutCreateInfo, PipelineShaderStageCreateInfo, Sampler,
        ShaderModuleCreateInfo, ShaderStageFlags, SpecializationInfo, SpecializationMapEntry,
        WriteDescriptorSet,
    },
};

use thiserror::Error;
use tonemap_output::TonemapOutput;
use tracing::info_span;
use vulkan_instance::VulkanInstance;

/// Tonemaps a capture from the scRGB colorspace into the sRGB colorspace.\
/// Returns a vulkan image containing the capture.
pub fn tonemap(
    vk: &VulkanInstance,
    capture: ImageView,
    capture_size: [u32; 2],
    whitepoint: f32,
) -> Result<TonemapOutput, Error> {
    let _span = info_span!("tonemap").entered();

    // Create output image
    let capture_output = TonemapOutput::new(vk, capture_size)?;

    let (shader_module, shader_entry_name) = unsafe {
        let mut shader_file = Cursor::new(&include_bytes!("shaders/scRGB_to_sRGB.spv")[..]);
        let shader_code = read_spv(&mut shader_file).map_err(Error::ReadShader)?;
        let shader_info = ShaderModuleCreateInfo::default().code(&shader_code);
        let shader_module = vk
            .device
            .create_shader_module(&shader_info, None)
            .map_err(|e| Error::Vulkan(e, "creating shader module"))?;
        let shader_entry_name = std::ffi::CStr::from_bytes_with_nul_unchecked(b"main\0");

        (shader_module, shader_entry_name)
    };

    // Shader sepcialization
    let whitepoint_specialization = SpecializationMapEntry::default()
        .constant_id(0)
        .offset(0)
        .size(4);
    let specialization_entries = [whitepoint_specialization];
    let specialization_data = whitepoint.to_le_bytes();
    let shader_specialization = SpecializationInfo::default()
        .map_entries(&specialization_entries)
        .data(&specialization_data);

    let shader_stage_create_info = PipelineShaderStageCreateInfo::default()
        .stage(ShaderStageFlags::COMPUTE)
        .module(shader_module)
        .name(shader_entry_name)
        .specialization_info(&shader_specialization);

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
            .map_err(|e| Error::Vulkan(e, "creating descriptor set layout"))?;

        [descriptor_layout]
    };

    let descriptor_sets = unsafe {
        let descriptor_allocate_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_layouts);

        vk.device
            .allocate_descriptor_sets(&descriptor_allocate_info)
            .map_err(|e| Error::Vulkan(e, "allocating descriptor sets"))?
    };

    unsafe {
        let input_image_descriptor = DescriptorImageInfo {
            sampler: Sampler::null(),
            image_view: capture,
            image_layout: ImageLayout::GENERAL,
        };

        let output_image_descriptor = DescriptorImageInfo {
            sampler: Sampler::null(),
            image_view: capture_output.image_view,
            image_layout: ImageLayout::GENERAL,
        };

        let write_descriptor_sets = [
            WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: DescriptorType::STORAGE_IMAGE,
                p_image_info: &input_image_descriptor,
                ..Default::default()
            },
            WriteDescriptorSet {
                dst_set: descriptor_sets[0],
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
            .create_compute_pipelines(PipelineCache::null(), &[compute_pipeline_create_info], None)
            .map_err(|(_, e)| Error::Vulkan(e, "creating compute pipline"))?;

        (compute_pipeline[0], pipeline_layout)
    };

    // Dispatch tonemapper
    let dispatch_span = info_span!("dispatch").entered();

    // Shader tonemaps a 32x32 area each dispatch
    let workgroup_x = capture_size[0].div_ceil(32);
    let workgroup_y = capture_size[1].div_ceil(32);

    vk.record_submit_command_buffer(
        vk.command_buffer,
        vk.fence,
        &[],
        &[],
        |device, command_buffer| unsafe {
            device.cmd_bind_descriptor_sets(
                command_buffer,
                PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );

            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::COMPUTE, compute_pipeline);

            device.cmd_dispatch(command_buffer, workgroup_x, workgroup_y, 1);
            Ok(())
        },
    )?;

    unsafe { vk.device.wait_for_fences(&[vk.fence], true, u64::MAX) }
        .map_err(|e| Error::Vulkan(e, "waiting for fence"))?;

    // cleanup resources
    unsafe {
        vk.device.destroy_pipeline_layout(pipeline_layout, None);
        vk.device.destroy_pipeline(compute_pipeline, None);

        vk.device
            .destroy_descriptor_set_layout(descriptor_layouts[0], None);
        vk.device.destroy_descriptor_pool(descriptor_pool, None);

        vk.device.destroy_shader_module(shader_module, None);
    }

    dispatch_span.exit();

    Ok(capture_output)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create tonemap output image:\n{0}")]
    TonemapOutput(#[from] tonemap_output::Error),

    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] vk::Result, &'static str),

    #[error("No suitable memory types are available for the allocation")]
    NoSuitableMemoryType,

    #[error("Failed to read shader:\n{0}")]
    ReadShader(#[source] io::Error),

    #[error("Failed to record and submit command buffer:\n{0}")]
    RecordSubmit(#[from] vulkan_instance::record_submit_command_buffer::Error),
}
