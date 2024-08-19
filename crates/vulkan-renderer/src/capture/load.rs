use ash::vk::{
    DescriptorImageInfo, DescriptorType, ImageLayout, ImageView, Sampler, WriteDescriptorSet,
};
use thiserror::Error;
use vulkan_instance::VulkanInstance;

use super::Capture;

impl Capture {
    pub fn load_capture(
        &mut self,
        vk: &VulkanInstance,
        image: ImageView,
        whitepoint: f32,
    ) -> Result<(), Error> {
        unsafe {
            let image_descriptor = DescriptorImageInfo {
                sampler: Sampler::null(),
                image_view: image,
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

        self.whitepoint = whitepoint;
        self.loaded = true;

        Ok(())
    }

    pub fn unload_capture(&mut self) {
        self.loaded = false;
    }
}

#[derive(Debug, Error)]
pub enum Error {}
