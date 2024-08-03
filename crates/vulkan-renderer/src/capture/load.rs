use std::sync::Arc;

use scrgb_tonemapper::tonemap_output::TonemapOutput;
use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::Pipeline,
    Validated, VulkanError,
};

use super::Capture;

impl Capture {
    pub fn load_capture(
        &mut self,
        vk: &VulkanInstance,
        texture: Arc<TonemapOutput>,
    ) -> Result<(), Error> {
        let ds_layout = self.pipeline.layout().set_layouts()[0].clone();

        let descriptor_set = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, texture.sampler.clone()),
                WriteDescriptorSet::image_view(1, texture.image_view.clone()),
            ],
            [],
        )?;

        self.capture = Some(texture);
        self.capture_ds = Some(descriptor_set);

        Ok(())
    }

    pub fn unload_capture(&mut self) {
        self.capture = None;
        self.capture_ds = None;
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[from] Validated<VulkanError>),
}
