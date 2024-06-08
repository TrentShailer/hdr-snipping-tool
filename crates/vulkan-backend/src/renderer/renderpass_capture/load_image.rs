use thiserror::Error;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::Pipeline,
    Validated, VulkanError,
};

use crate::{texture::Texture, VulkanInstance};

use super::RenderpassCapture;

impl RenderpassCapture {
    pub fn load_image(&mut self, vulkan: &VulkanInstance, texture: Texture) -> Result<(), Error> {
        let ds_layout = self.pipeline.layout().set_layouts()[0].clone();

        let descriptor_set = PersistentDescriptorSet::new(
            &vulkan.allocators.descriptor,
            ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, texture.sampler.clone()),
                WriteDescriptorSet::image_view(1, texture.image_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        self.capture = Some(texture);
        self.capture_ds = Some(descriptor_set);

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create descriptor set")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
