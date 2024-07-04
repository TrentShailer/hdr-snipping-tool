use half::f16;
use vulkan_instance::VulkanInstance;

use crate::{glyph_cache::GlyphCache, text::set_text};

use super::Parameters;

impl Parameters {
    pub fn set_parameters(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
        alpha: f16,
        gamma: f16,
        maximum: f16,
    ) -> Result<(), set_text::Error> {
        let text = format!(
            "Gamma: {:.2}\nAlpha: {:.2}\nInMax: {:.2}",
            gamma, alpha, maximum
        );

        self.text.set_text(vk, glyph_cache, &text)?;

        Ok(())
    }
}
