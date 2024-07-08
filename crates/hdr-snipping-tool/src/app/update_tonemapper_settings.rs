use thiserror::Error;
use vulkan_renderer::text;
use vulkan_tonemapper::tonemapper;

use super::{ActiveApp, ActiveCapture};

impl ActiveCapture {
    /// Increments the alpha and gamma values of the tonemapper, tonemaps, and then updates the UI
    pub fn update_tonemapper_settings(
        &mut self,
        app: &mut ActiveApp,
        alpha_increment: f32,
        gamma_increment: f32,
    ) -> Result<(), Error> {
        self.tonemapper.config.gamma += gamma_increment;
        self.tonemapper.config.alpha += alpha_increment;

        self.tonemapper.tonemap(&app.vulkan_instance)?;

        app.renderer.parameters.set_parameters(
            &app.vulkan_instance,
            &mut app.renderer.glyph_cache,
            self.tonemapper.config.alpha,
            self.tonemapper.config.gamma,
            self.tonemapper.config.maximum,
        )?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] tonemapper::tonemap::Error),

    #[error("Failed to update UI:\n{0}")]
    UpdateUI(#[from] text::set_text::Error),
}
