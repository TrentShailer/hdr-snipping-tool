pub mod drop;
pub mod load;
pub mod render;

use ash::vk::{
    Buffer, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo, DescriptorPoolSize,
    DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorType, DeviceMemory,
    Filter, ImageLayout, ImageView, Pipeline, PipelineLayout, Sampler, SamplerAddressMode,
    SamplerCreateInfo, WriteDescriptorSet,
};
use thiserror::Error;
use vulkan_instance::VulkanInstance;

use crate::{pipelines::capture::Vertex, vertex_index_buffer::create_vertex_and_index_buffer};

pub struct Capture {
    pub vertex_buffer: (Buffer, DeviceMemory),
    pub index_buffer: (Buffer, DeviceMemory),
    pub indicies: u32,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: Pipeline,

    pub sampler: Sampler,

    pub descriptor_pool: DescriptorPool,
    pub descriptor_sets: Vec<DescriptorSet>,

    pub loaded: bool,
    pub whitepoint: f32,
}

impl Capture {
    pub fn new(
        vk: &VulkanInstance,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        descriptor_layouts: [DescriptorSetLayout; 2],
    ) -> Result<Self, Error> {
        let verticies = vec![
            Vertex {
                position: [-1.0, -1.0],
                uv: [0.0, 0.0],
            }, // TL
            Vertex {
                position: [1.0, -1.0],
                uv: [1.0, 0.0],
            }, // TR
            Vertex {
                position: [1.0, 1.0],
                uv: [1.0, 1.0],
            }, // BR
            Vertex {
                position: [-1.0, 1.0],
                uv: [0.0, 1.0],
            }, // BL
        ];

        let indicies = vec![0, 1, 2, 2, 3, 0];

        let (vertex_buffer, index_buffer) =
            create_vertex_and_index_buffer(vk, &verticies, &indicies)?;

        let sampler_create_info = SamplerCreateInfo::default()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .address_mode_u(SamplerAddressMode::REPEAT)
            .address_mode_v(SamplerAddressMode::REPEAT)
            .address_mode_w(SamplerAddressMode::REPEAT);
        let sampler = unsafe { vk.device.create_sampler(&sampler_create_info, None) }
            .map_err(|e| Error::Vulkan(e, "creating sampler"))?;

        //
        let descriptor_pool = unsafe {
            let descriptor_sizes = [
                DescriptorPoolSize {
                    ty: DescriptorType::SAMPLER,
                    descriptor_count: 1,
                },
                DescriptorPoolSize {
                    ty: DescriptorType::SAMPLED_IMAGE,
                    descriptor_count: 1,
                },
            ];
            let descriptor_pool_info = DescriptorPoolCreateInfo::default()
                .pool_sizes(&descriptor_sizes)
                .max_sets(2);

            vk.device
                .create_descriptor_pool(&descriptor_pool_info, None)
                .map_err(|e| Error::Vulkan(e, "creating descriptor pool"))?
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
            let sampler_descriptor = DescriptorImageInfo {
                sampler,
                image_view: ImageView::null(),
                image_layout: ImageLayout::GENERAL,
            };

            let write_descriptor_sets = [WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: DescriptorType::SAMPLER,
                p_image_info: &sampler_descriptor,
                ..Default::default()
            }];

            vk.device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        Ok(Self {
            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            pipeline_layout,
            pipeline,

            sampler,

            descriptor_pool,
            descriptor_sets,

            loaded: false,
            whitepoint: 0.0,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create vertex and index buffer:\n{0}")]
    CreateVertexIndexBuffer(#[from] crate::vertex_index_buffer::Error),

    #[error("Encountered vulkan error while {1}:\n{0}")]
    Vulkan(#[source] ash::vk::Result, &'static str),
}
