use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    image::{view::ImageView, Image},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    Validated, VulkanError,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),

    #[error("Failed to create framebuffer:\n{0:?}")]
    CreateFramebuffer(#[source] Validated<VulkanError>),
}

/// This function is called once during initialization, then again whenever the window is resized.
pub fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Result<Vec<Arc<Framebuffer>>, Error> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).map_err(Error::ImageView)?;
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .map_err(Error::CreateFramebuffer)
        })
        .collect::<Result<Vec<_>, _>>()
}
