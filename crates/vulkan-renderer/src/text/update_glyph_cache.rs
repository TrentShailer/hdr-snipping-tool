use thiserror::Error;
use vulkan_instance::VulkanInstance;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    pipeline::Pipeline,
    Validated, VulkanError,
};

use crate::glyph_cache::GlyphCache;

use super::{set_text, Text};

impl Text {
    pub fn update_glyph_cache(
        &mut self,
        vk: &VulkanInstance,
        new_glyph_cache: &mut GlyphCache,
    ) -> Result<(), Error> {
        let atlas_ds_layout = self.pipeline.layout().set_layouts()[0].clone();
        let atlas_ds = PersistentDescriptorSet::new(
            &vk.allocators.descriptor,
            atlas_ds_layout.clone(),
            [
                WriteDescriptorSet::sampler(0, new_glyph_cache.atlas_sampler.clone()),
                WriteDescriptorSet::image_view(1, new_glyph_cache.atlas_view.clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;
        self.atlas_ds = atlas_ds;

        self.set_text(vk, new_glyph_cache, &self.text.clone())?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to reset text:\n{0}")]
    SetText(#[from] set_text::Error),

    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[from] Validated<VulkanError>),
}
