use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::view::ImageView,
    pipeline::Pipeline,
    Validated, VulkanError,
};

use super::Capture;

impl Capture {
    pub fn load_capture(
        &mut self,
        vk: &VulkanInstance,
        image: Arc<ImageView>,
        whitepoint: f32,
    ) -> Result<(), Error> {
        let ds_layout = self.pipeline.layout().set_layouts()[0].clone();

        let descriptor_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, self.sampler.clone()),
                WriteDescriptorSet::image_view(1, image.clone()),
            ],
            [],
        )?;

        self.whitepoint = whitepoint;
        self.capture_ds = Some(descriptor_set);

        Ok(())
    }

    pub fn unload_capture(&mut self) {
        self.capture_ds = None;
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[from] Validated<VulkanError>),
}
