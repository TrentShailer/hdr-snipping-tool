pub mod find_maximum;
pub mod tonemap;
pub mod whitepoint;

use std::{fmt::Debug, sync::Arc};

use scrgb::ScRGB;
use thiserror::Error;
use vulkan_instance::{
    copy_buffer::{self, copy_buffer_and_wait},
    VulkanInstance,
};
use vulkano::{
    buffer::{AllocateBufferError, Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{
        compute::ComputePipelineCreateInfo,
        layout::{IntoPipelineLayoutCreateInfoError, PipelineDescriptorSetLayoutCreateInfo},
        ComputePipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    sync::HostAccessError,
    Validated, VulkanError,
};
use windows_capture_provider::{capture::Capture, display::Display};

use crate::{find_maximum::find_maximum, whitepoint::Whitepoint};

mod shader {
    vulkano_shaders::shader! {ty: "compute", bytes: "src/shaders/scRGB_to_sRGB.spv"}
}

/// A tonemapper setup to tonemap from the scRGB color space to the sRGB color space.
pub struct ScrgbTonemapper {
    /// The whitepoint to control the tonemapping curve.
    pub curve_target: Whitepoint,

    /// The display metadata from the capture.
    display: Display,

    /// The value of the brightest component in the capture.
    brightest_component: ScRGB,

    pipeline: Arc<ComputePipeline>,
    io_set: Arc<PersistentDescriptorSet>,
}

impl ScrgbTonemapper {
    /// Creates a new tonemapper for a given capture.
    pub fn new(
        vk: &VulkanInstance,
        output_view: Arc<ImageView>,
        capture: &Capture,
    ) -> Result<Self, Error> {
        let staging_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            capture.data.len() as u64,
        )?;
        staging_buffer.write()?.copy_from_slice(&capture.data);

        // find the brightest component in the capture.
        let brightest_component =
            find_maximum(vk, staging_buffer.clone(), capture.data.len() as u32)?.to_f32();
        let brightest_component = ScRGB(brightest_component);

        let input_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            vk.allocators.memory.clone(),
            BufferCreateInfo {
                usage: BufferUsage::STORAGE_BUFFER | BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
            capture.data.len() as u64,
        )?;

        copy_buffer_and_wait(
            vk,
            staging_buffer.clone(),
            input_buffer.clone(),
            vulkan_instance::copy_buffer::Region::SmallestBuffer,
        )?;

        let pipeline = {
            let compute_shader = shader::load(vk.device.clone())
                .map_err(Error::LoadShader)?
                .entry_point("main")
                .unwrap();

            let stage = PipelineShaderStageCreateInfo::new(compute_shader);

            let layout = PipelineLayout::new(
                vk.device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
                    .into_pipeline_layout_create_info(vk.device.clone())?,
            )
            .map_err(Error::CreatePipelineLayout)?;

            ComputePipeline::new(
                vk.device.clone(),
                None,
                ComputePipelineCreateInfo::stage_layout(stage, layout),
            )
            .map_err(Error::CreatePipeline)?
        };
        let io_layout = &pipeline.layout().set_layouts()[0];
        let io_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            io_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, input_buffer.clone()),
                WriteDescriptorSet::image_view(1, output_view.clone()),
            ],
            [],
        )
        .map_err(Error::Descriptor)?;

        let tonemapper = Self {
            curve_target: Whitepoint::SdrReferenceWhite,
            display: capture.display,
            brightest_component,
            io_set,
            pipeline,
        };

        log::debug!(
            "Tonemapper {{curve_target: {:?}, display: {:?}, brightest_component: {:?}}}",
            tonemapper.curve_target,
            tonemapper.display.handle,
            tonemapper.brightest_component
        );

        Ok(tonemapper)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to allocate buffer:\n{0:?}")]
    AllocateBuffer(#[from] Validated<AllocateBufferError>),

    #[error("Failed to access buffer:\n{0:?}")]
    BufferAccess(#[from] HostAccessError),

    #[error("Failed to copy buffer:\n{0}")]
    CopyBuffer(#[from] copy_buffer::Error),

    #[error("Failed to load shader:\n{0:?}")]
    LoadShader(#[source] Validated<VulkanError>),

    #[error("Failed to create pipeline layout info:\n{0:?}")]
    CreatePipelineLayoutInfo(#[from] IntoPipelineLayoutCreateInfoError),

    #[error("Failed to create pipeline layout:\n{0:?}")]
    CreatePipelineLayout(#[source] Validated<VulkanError>),

    #[error("Failed to create pipline:\n{0:?}")]
    CreatePipeline(#[source] Validated<VulkanError>),

    #[error("Failed to create descriptor set:\n{0:?}")]
    Descriptor(#[source] Validated<VulkanError>),

    #[error("Failed to find capture maximum:\n{0}")]
    Maximum(#[from] find_maximum::Error),
}
