use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    format::Format,
    image::{view::ImageView, AllocateImageError, Image, ImageCreateInfo, ImageUsage},
    memory::allocator::AllocationCreateInfo,
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    Validated, VulkanError,
};

use crate::VulkanInstance;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to get image view:\n{0:?}")]
    ImageView(#[source] Validated<VulkanError>),

    #[error("Failed to create framebuffer:\n{0:?}")]
    CreateFramebuffer(#[source] Validated<VulkanError>),

    #[error("Failed to create image:\n{0:?}")]
    CreateImage(#[from] Validated<AllocateImageError>),
}

fn transient_image(
    instance: &VulkanInstance,
    extent: [u32; 3],
) -> Result<(Arc<Image>, Arc<ImageView>), Error> {
    let image = Image::new(
        instance.allocators.memory.clone(),
        ImageCreateInfo {
            extent,
            usage: ImageUsage::TRANSIENT_ATTACHMENT
                | ImageUsage::COLOR_ATTACHMENT
                | ImageUsage::INPUT_ATTACHMENT,
            format: Format::R8G8B8A8_SRGB,
            ..Default::default()
        },
        AllocationCreateInfo::default(),
    )?;

    let view = ImageView::new_default(image.clone()).map_err(Error::ImageView)?;

    Ok((image, view))
}

/// This function is called once during initialization, then again whenever the window is resized.
pub fn window_size_dependent_setup(
    vulkan: &VulkanInstance,
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Result<(Vec<Arc<Framebuffer>>, Arc<[Arc<ImageView>; 4]>), Error> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];

    let (_capture_out, capture_view) = transient_image(&vulkan, extent)?;
    let (_selection_out, selection_view) = transient_image(&vulkan, extent)?;
    let (_border_out, border_view) = transient_image(&vulkan, extent)?;
    let (_mouse_out, mouse_view) = transient_image(&vulkan, extent)?;

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).map_err(Error::ImageView)?;
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        view,
                        capture_view.clone(),
                        selection_view.clone(),
                        border_view.clone(),
                        mouse_view.clone(),
                    ],
                    ..Default::default()
                },
            )
            .map_err(Error::CreateFramebuffer)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((
        framebuffers,
        Arc::new([capture_view, selection_view, border_view, mouse_view]),
    ))
}
