use core::slice;

use ash::vk::{self, ImageViewType};
use ash_helper::{
    allocate_image, cmd_transition_image, cmd_try_begin_label, cmd_try_end_label, onetime_command,
    VkError, VulkanContext,
};
use bytemuck::bytes_of;

use crate::{HdrImage, QueuePurpose, SdrImage};

use super::{TonemapperError, HdrToSdrTonemapper, PushConstants};

impl HdrToSdrTonemapper {
    /// Runs the HDR to SDR tonemapper over an input image.
    ///
    /// ## Whitepoint
    /// Whitepoint is the maximum brightness that a colour component is clamped to. This prevents
    /// extreme values in the input from underexposing the output image.
    ///
    /// ## Input Image Requirements
    /// * `format: R16G16B16A16_SFLOAT`
    /// * `layout: GENERAL`
    /// * `usage: STORAGE`
    ///
    /// ## Output Image
    /// * `format: R8G8B8A8_UNORM`
    /// * `layout: GENERAL`
    /// * `usage: STORAGE, TRANSFER_SRC`
    pub unsafe fn tonemap(&self, hdr_image: HdrImage, whitepoint: f32) -> Result<SdrImage, TonemapperError> {
        // Create the output image
        let (sdr_image, sdr_memory) = {
            let queue_family = self.vulkan.queue_family_index();

            let create_info = vk::ImageCreateInfo::default()
                .array_layers(1)
                .extent(hdr_image.extent.into())
                .format(vk::Format::R8G8B8A8_UNORM)
                .image_type(vk::ImageType::TYPE_2D)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .mip_levels(1)
                .queue_family_indices(slice::from_ref(&queue_family))
                .samples(vk::SampleCountFlags::TYPE_1)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .tiling(vk::ImageTiling::OPTIMAL)
                .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC);

            let (image, memory, _) = unsafe {
                allocate_image(
                    self.vulkan.as_ref(),
                    &create_info,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                    "HDR to SDR Tonemapper Output",
                )?
            };

            (image, memory)
        };

        // Create the image view
        let output_image_view = {
            let image_view_create_info = vk::ImageViewCreateInfo::default()
                .image(sdr_image)
                .view_type(ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_UNORM)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1)
                        .level_count(1),
                );

            self.vulkan
                .device()
                .create_image_view(&image_view_create_info, None)
                .map_err(|e| VkError::new(e, "vkCreateImageView"))?
        };

        // Descriptors for the images
        let input_descriptor = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(hdr_image.view)
            .sampler(vk::Sampler::null());

        let output_descriptor = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::GENERAL)
            .image_view(output_image_view)
            .sampler(vk::Sampler::null());

        // Run the shader
        onetime_command(
            self.vulkan.as_ref(),
            self.vulkan.transient_pool(),
            self.vulkan.queue(QueuePurpose::Compute),
            |vk, command_buffer| {
                cmd_try_begin_label(vk, command_buffer, "HDR to SDR Tonemap");

                // Transition output to general
                cmd_transition_image(
                    vk,
                    command_buffer,
                    sdr_image,
                    vk::ImageLayout::UNDEFINED,
                    vk::ImageLayout::GENERAL,
                )
                .unwrap();

                // Push descriptor writes
                {
                    let descriptor_writes = [
                        // Input
                        vk::WriteDescriptorSet::default()
                            .dst_set(vk::DescriptorSet::null())
                            .dst_binding(0)
                            .descriptor_count(1)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .image_info(slice::from_ref(&input_descriptor)),
                        // Output
                        vk::WriteDescriptorSet::default()
                            .dst_set(vk::DescriptorSet::null())
                            .dst_binding(1)
                            .descriptor_count(1)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .image_info(slice::from_ref(&output_descriptor)),
                    ];

                    vk.push_descriptor_device().cmd_push_descriptor_set(
                        command_buffer,
                        vk::PipelineBindPoint::COMPUTE,
                        self.layout,
                        0,
                        &descriptor_writes,
                    );
                }

                // Push whitepoint
                {
                    let push_constants = PushConstants { whitepoint };
                    vk.device().cmd_push_constants(
                        command_buffer,
                        self.layout,
                        vk::ShaderStageFlags::COMPUTE,
                        0,
                        bytes_of(&push_constants),
                    );
                }

                // Bind the pipeline
                vk.device().cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    self.pipeline,
                );

                // Calculate the dispatches
                let dispatches = [
                    hdr_image.extent.width.div_ceil(16),
                    hdr_image.extent.height.div_ceil(16),
                ];

                // Dispatch
                vk.device()
                    .cmd_dispatch(command_buffer, dispatches[0], dispatches[1], 1);

                cmd_try_end_label(vk, command_buffer);
            },
            "HDR to SDR Tonemap",
        )?;

        // Clean up
        unsafe {
            self.vulkan
                .device()
                .destroy_image_view(output_image_view, None);
        }

        Ok(SdrImage {
            image: sdr_image,
            memory: sdr_memory,
            extent: hdr_image.extent,
        })
    }
}
