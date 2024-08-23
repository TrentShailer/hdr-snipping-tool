use ash::{
    vk::{
        Buffer, CommandBuffer, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo,
        DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorSetLayout,
        DescriptorType, DeviceMemory, Filter, ImageLayout, ImageView, IndexType, Pipeline,
        PipelineBindPoint, PipelineLayout, Sampler, SamplerAddressMode, SamplerCreateInfo,
        ShaderStageFlags, WriteDescriptorSet,
    },
    Device,
};
use bytemuck::bytes_of;
use hdr_capture::HdrCapture;
use tracing::{instrument, Level};
use vulkan_instance::{VulkanError, VulkanInstance};

use crate::pipelines::{
    capture::{PushConstants, Vertex},
    vertex_index_buffer::create_vertex_and_index_buffer,
};

pub struct Capture<'d> {
    device: &'d Device,

    vertex_buffer: (Buffer, DeviceMemory),
    index_buffer: (Buffer, DeviceMemory),
    indicies: u32,

    pipeline_layout: PipelineLayout,
    pipeline: Pipeline,

    sampler: Sampler,

    descriptor_pool: DescriptorPool,
    descriptor_sets: Vec<DescriptorSet>,

    push_constants: PushConstants,
    pub loaded: bool,
}

impl<'d> Capture<'d> {
    #[instrument("Capture::new", skip_all, err)]
    pub fn new(
        vk: &'d VulkanInstance,
        pipeline: Pipeline,
        pipeline_layout: PipelineLayout,
        descriptor_layouts: [DescriptorSetLayout; 2],
    ) -> Result<Self, crate::Error> {
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
            .map_err(|e| VulkanError::VkResult(e, "creating sampler"))?;

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
                .map_err(|e| VulkanError::VkResult(e, "creating descriptor pool"))?
        };

        let descriptor_sets = unsafe {
            let descriptor_allocate_info = DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_layouts);

            vk.device
                .allocate_descriptor_sets(&descriptor_allocate_info)
                .map_err(|e| VulkanError::VkResult(e, "allocating descriptor sets"))?
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

        let push_constants = PushConstants { whitepoint: 0.0 };

        Ok(Self {
            device: &vk.device,

            vertex_buffer,
            index_buffer,
            indicies: indicies.len() as u32,

            pipeline_layout,
            pipeline,

            sampler,

            descriptor_pool,
            descriptor_sets,

            loaded: false,
            push_constants,
        })
    }

    #[instrument("Capture::load_capture", level = Level::DEBUG, skip_all, err)]
    pub fn load_capture(
        &mut self,
        vk: &VulkanInstance,
        capture: &HdrCapture,
    ) -> Result<(), crate::Error> {
        unsafe {
            let image_descriptor = DescriptorImageInfo {
                sampler: Sampler::null(),
                image_view: capture.image_view,
                image_layout: ImageLayout::GENERAL,
            };

            let write_descriptor_sets = [WriteDescriptorSet {
                dst_set: self.descriptor_sets[1],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: DescriptorType::SAMPLED_IMAGE,
                p_image_info: &image_descriptor,
                ..Default::default()
            }];

            vk.device
                .update_descriptor_sets(&write_descriptor_sets, &[]);
        };

        self.push_constants.whitepoint = capture.whitepoint;
        self.loaded = true;

        Ok(())
    }

    pub fn unload_capture(&mut self) {
        self.loaded = false;
    }

    #[instrument("Capture::render", level = Level::DEBUG, skip_all, err)]
    pub fn render(
        &self,
        device: &Device,
        command_buffer: CommandBuffer,
    ) -> Result<(), ash::vk::Result> {
        if !self.loaded {
            return Ok(());
        }

        unsafe {
            device.cmd_bind_pipeline(command_buffer, PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.0], &[0]);
            device.cmd_bind_index_buffer(command_buffer, self.index_buffer.0, 0, IndexType::UINT32);
            device.cmd_bind_descriptor_sets(
                command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets,
                &[],
            );
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                ShaderStageFlags::FRAGMENT,
                0,
                bytes_of(&self.push_constants),
            );

            device.cmd_draw_indexed(command_buffer, self.indicies, 1, 0, 0, 0);
        }

        Ok(())
    }
}

impl<'d> Drop for Capture<'d> {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.vertex_buffer.0, None);
            self.device.free_memory(self.vertex_buffer.1, None);
            self.device.destroy_buffer(self.index_buffer.0, None);
            self.device.free_memory(self.index_buffer.1, None);

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_sampler(self.sampler, None);
        }
    }
}
