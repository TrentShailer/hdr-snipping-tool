use std::sync::Arc;

use thiserror::Error;
use vulkan_instance::{texture::Texture, VulkanInstance};
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::Pipeline,
    Validated, VulkanError,
};

use crate::Renderer;

impl Renderer {
    pub fn load_texture(
        &mut self,
        vk: &VulkanInstance,
        texture: Arc<Texture>,
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
        )
        .map_err(Error::DescriptorSet)?;

        self.texture = Some(texture);
        self.texture_ds = Some(descriptor_set);

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
