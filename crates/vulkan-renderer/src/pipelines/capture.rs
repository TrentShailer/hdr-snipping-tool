use std::{
    mem::{offset_of, size_of},
    sync::Arc,
};

use ash::vk::{
    BlendFactor, BlendOp, ColorComponentFlags, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateInfo, DescriptorType, Format, Pipeline,
    PipelineColorBlendAttachmentState, PipelineLayout, PipelineLayoutCreateInfo,
    PipelineRenderingCreateInfo, PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PushConstantRange, ShaderModule, ShaderStageFlags, VertexInputAttributeDescription,
    VertexInputBindingDescription, VertexInputRate, Viewport,
};
use bytemuck::{Pod, Zeroable};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use super::MAIN_CSTR;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstants {
    pub whitepoint: f32,
    pub flags: u32,
}

pub const ENCODE_AS_SRGB_FLAG: u32 = 0b1;

pub struct CapturePipeline {
    vk: Arc<VulkanInstance>,
    pub pipeline: Pipeline,
    pub layout: PipelineLayout,
    pub descriptor_layouts: [DescriptorSetLayout; 2],
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
}

impl CapturePipeline {
    #[instrument("CapturePipeline::new", skip_all, err)]
    pub fn new(
        vk: Arc<VulkanInstance>,
        pipeline_rendering_create_info: PipelineRenderingCreateInfo,
        viewport: Viewport,
    ) -> Result<Self, VulkanError> {
        let (vertex_input_binding_descriptions, vertex_attribute_descriptions) =
            Self::vertex_descriptions();
        let vertex_input_info = PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_attribute_descriptions[..])
            .vertex_binding_descriptions(&vertex_input_binding_descriptions[..]);

        let vertex_shader =
            vk.create_shader_module(include_bytes!("../shaders/capture.vert.spv"))?;
        let fragment_shader =
            vk.create_shader_module(include_bytes!("../shaders/capture.frag.spv"))?;

        let shader_stage_infos = [
            PipelineShaderStageCreateInfo::default()
                .stage(ShaderStageFlags::VERTEX)
                .module(vertex_shader)
                .name(MAIN_CSTR),
            PipelineShaderStageCreateInfo::default()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(fragment_shader)
                .name(MAIN_CSTR),
        ];

        let blend = Self::blend();

        let push_constant_ranges = [PushConstantRange {
            stage_flags: ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: size_of::<PushConstants>() as u32,
        }];

        let descriptor_layouts = unsafe {
            let sampler_layout = {
                let descriptor_layout_bindings = [DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::SAMPLER,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                }];

                let descriptor_info =
                    DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_layout_bindings);

                vk.device
                    .create_descriptor_set_layout(&descriptor_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating descriptor set layout"))?
            };

            let view_layout = {
                let descriptor_layout_bindings = [DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: DescriptorType::SAMPLED_IMAGE,
                    descriptor_count: 1,
                    stage_flags: ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                }];

                let descriptor_info =
                    DescriptorSetLayoutCreateInfo::default().bindings(&descriptor_layout_bindings);

                vk.device
                    .create_descriptor_set_layout(&descriptor_info, None)
                    .map_err(|e| VulkanError::VkResult(e, "creating descriptor set layout"))?
            };

            [sampler_layout, view_layout]
        };

        let pipeline_layout_create_info = PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_layouts);

        let layout = unsafe {
            vk.device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
        }
        .map_err(|e| VulkanError::VkResult(e, "creating pipeline layout"))?;

        let pipeline = super::create_pipeline(
            &vk,
            pipeline_rendering_create_info,
            layout,
            vertex_input_info,
            &shader_stage_infos,
            blend,
            viewport,
        )?;

        Ok(Self {
            vk,
            pipeline,
            layout,
            descriptor_layouts,
            vertex_shader,
            fragment_shader,
        })
    }

    #[instrument("CapturePipeline::recreate", skip_all, err)]
    pub fn recreate(
        &mut self,
        pipeline_rendering_create_info: PipelineRenderingCreateInfo,
        viewport: Viewport,
    ) -> Result<(), VulkanError> {
        unsafe {
            self.vk.device.destroy_pipeline(self.pipeline, None);
        }

        let (vertex_input_binding_descriptions, vertex_attribute_descriptions) =
            Self::vertex_descriptions();
        let vertex_input_info = PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&vertex_attribute_descriptions[..])
            .vertex_binding_descriptions(&vertex_input_binding_descriptions[..]);

        let shader_stage_infos = [
            PipelineShaderStageCreateInfo::default()
                .stage(ShaderStageFlags::VERTEX)
                .module(self.vertex_shader)
                .name(MAIN_CSTR),
            PipelineShaderStageCreateInfo::default()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(self.fragment_shader)
                .name(MAIN_CSTR),
        ];

        let blend = Self::blend();

        let pipeline = super::create_pipeline(
            &self.vk,
            pipeline_rendering_create_info,
            self.layout,
            vertex_input_info,
            &shader_stage_infos,
            blend,
            viewport,
        )?;

        self.pipeline = pipeline;

        Ok(())
    }

    fn vertex_descriptions() -> (
        [VertexInputBindingDescription; 1],
        [VertexInputAttributeDescription; 2],
    ) {
        let vertex_input_binding_descriptions = [VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: VertexInputRate::VERTEX,
        }];

        let vertex_attribute_descriptions = [
            VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, position) as u32,
            },
            VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
        ];

        (
            vertex_input_binding_descriptions,
            vertex_attribute_descriptions,
        )
    }

    fn blend() -> PipelineColorBlendAttachmentState {
        PipelineColorBlendAttachmentState {
            blend_enable: 1,
            src_color_blend_factor: BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: BlendOp::ADD,
            src_alpha_blend_factor: BlendFactor::SRC_ALPHA,
            dst_alpha_blend_factor: BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: BlendOp::ADD,
            color_write_mask: ColorComponentFlags::RGBA,
        }
    }
}

impl Drop for CapturePipeline {
    fn drop(&mut self) {
        unsafe {
            if self.vk.device.device_wait_idle().is_err() {
                return;
            }

            self.vk.device.destroy_pipeline(self.pipeline, None);
            self.vk.device.destroy_pipeline_layout(self.layout, None);
            self.vk
                .device
                .destroy_descriptor_set_layout(self.descriptor_layouts[0], None);
            self.vk
                .device
                .destroy_descriptor_set_layout(self.descriptor_layouts[1], None);
            self.vk
                .device
                .destroy_shader_module(self.vertex_shader, None);
            self.vk
                .device
                .destroy_shader_module(self.fragment_shader, None);
        }
    }
}
