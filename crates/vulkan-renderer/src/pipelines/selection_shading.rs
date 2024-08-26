use std::{
    mem::{offset_of, size_of},
    sync::Arc,
};

use ash::vk::{
    BlendFactor, BlendOp, ColorComponentFlags, Format, Pipeline, PipelineColorBlendAttachmentState,
    PipelineLayout, PipelineLayoutCreateInfo, PipelineRenderingCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo, PushConstantRange,
    ShaderModule, ShaderStageFlags, VertexInputAttributeDescription, VertexInputBindingDescription,
    VertexInputRate, Viewport,
};
use bytemuck::{Pod, Zeroable};
use tracing::instrument;
use vulkan_instance::{VulkanError, VulkanInstance};

use super::MAIN_CSTR;

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [u8; 4],
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstants {
    pub base_position: [f32; 2],
    pub base_size: [f32; 2],
    pub target_position: [f32; 2],
    pub target_size: [f32; 2],
}

pub struct SelectionShadingPipeline {
    vk: Arc<VulkanInstance>,
    pub pipeline: Pipeline,
    pub layout: PipelineLayout,
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
}

impl SelectionShadingPipeline {
    #[instrument("SelectionShadingPipeline::new", skip_all, err)]
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
            vk.create_shader_module(include_bytes!("../shaders/selection_shading.vert.spv"))?;
        let fragment_shader =
            vk.create_shader_module(include_bytes!("../shaders/selection_shading.frag.spv"))?;

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
            stage_flags: ShaderStageFlags::VERTEX,
            offset: 0,
            size: size_of::<PushConstants>() as u32,
        }];

        let pipeline_layout_create_info =
            PipelineLayoutCreateInfo::default().push_constant_ranges(&push_constant_ranges);

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
            vertex_shader,
            fragment_shader,
        })
    }

    #[instrument("SelectionShadingPipeline::recreate", skip_all, err)]
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
        [VertexInputAttributeDescription; 3],
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
                format: Format::R8G8B8A8_UNORM,
                offset: offset_of!(Vertex, color) as u32,
            },
            VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: Format::R32_UINT,
                offset: offset_of!(Vertex, flags) as u32,
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

impl Drop for SelectionShadingPipeline {
    fn drop(&mut self) {
        unsafe {
            if self.vk.device.device_wait_idle().is_err() {
                return;
            }

            self.vk.device.destroy_pipeline(self.pipeline, None);
            self.vk.device.destroy_pipeline_layout(self.layout, None);
            self.vk
                .device
                .destroy_shader_module(self.vertex_shader, None);
            self.vk
                .device
                .destroy_shader_module(self.fragment_shader, None);
        }
    }
}
