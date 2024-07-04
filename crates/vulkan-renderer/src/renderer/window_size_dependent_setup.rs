use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    image::{view::ImageView, Image},
    pipeline::graphics::viewport::Viewport,
    Validated, VulkanError,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),
}

/// This function is called once during initialization, then again whenever the window is resized.
pub fn window_size_dependent_setup(
    images: &[Arc<Image>],
    viewport: &mut Viewport,
) -> Result<Vec<Arc<ImageView>>, Error> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).map_err(Error::ImageView))
        .collect::<Result<Vec<_>, _>>()
}
