use scrgb::ScRGB;
use scrgb_tonemapper::whitepoint::Whitepoint;
use thiserror::Error;

use super::ActiveApp;

impl ActiveApp {
    pub fn adjust_whitepoint(&mut self, amount: ScRGB) -> Result<(), Error> {
        let Some(capture) = self.active_capture.as_mut() else {
            return Ok(());
        };
        let mut amount = amount;
        if self.keyboard_modifiers.shift_key() {
            amount *= 10.0;
        }

        capture.adjust_whitepoint(&self.vk, amount)?;

        self.renderer.parameters.set_parameters(
            &self.vk,
            &mut self.renderer.glyph_cache,
            capture.tonemapper.curve_target,
            capture.tonemapper.get_whitepoint(),
        )?;

        Ok(())
    }

    pub fn set_whitepoint(&mut self, whitepoint: Whitepoint) -> Result<(), Error> {
        let Some(capture) = self.active_capture.as_mut() else {
            return Ok(());
        };

        capture.set_whitepoint(&self.vk, whitepoint)?;

        self.renderer.parameters.set_parameters(
            &self.vk,
            &mut self.renderer.glyph_cache,
            capture.tonemapper.curve_target,
            capture.tonemapper.get_whitepoint(),
        )?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to tonemap:\n{0}")]
    Tonemap(#[from] scrgb_tonemapper::tonemap::Error),

    #[error("Failed to update text:\n{0}")]
    UpdateText(#[from] vulkan_renderer::text::set_text::Error),
}
