use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    image::view::ImageView,
    pipeline::{GraphicsPipeline, Pipeline},
    Validated, VulkanError,
};

use crate::VulkanInstance;

use super::RenderpassFinal;

impl RenderpassFinal {
    pub fn recreate_attachment_set(
        instance: &VulkanInstance,
        pipeline: Arc<GraphicsPipeline>,
        attachments: Arc<[Arc<ImageView>; 4]>,
    ) -> Result<Arc<PersistentDescriptorSet>, Error> {
        let layout = pipeline.layout().set_layouts().get(0).unwrap();
        let descriptor_set = PersistentDescriptorSet::new(
            &instance.allocators.descriptor,
            layout.clone(),
            [
                WriteDescriptorSet::image_view(0, attachments[0].clone()),
                WriteDescriptorSet::image_view(1, attachments[1].clone()),
                WriteDescriptorSet::image_view(2, attachments[2].clone()),
                WriteDescriptorSet::image_view(3, attachments[3].clone()),
            ],
            [],
        )
        .map_err(Error::DescriptorSet)?;

        Ok(descriptor_set)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create descriptor set:\n{0:?}")]
    DescriptorSet(#[source] Validated<VulkanError>),
}
