use scrgb::ScRGB;
use vulkan_instance::VulkanInstance;

use crate::{glyph_cache::GlyphCache, text::set_text};

use super::Parameters;

impl Parameters {
    pub fn clear_parameters(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
    ) -> Result<(), set_text::Error> {
        self.whitepoint = ScRGB(0.0);
        self.update_text(vk, glyph_cache)
    }

    pub fn set_whitepoint(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
        whitepoint: ScRGB,
    ) -> Result<(), set_text::Error> {
        self.whitepoint = whitepoint;
        self.update_text(vk, glyph_cache)
    }

    fn update_text(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
    ) -> Result<(), set_text::Error> {
        let text = format!("Whitepoint: {} nits", self.whitepoint.as_nits_string());
        self.text.set_text(vk, glyph_cache, &text)?;

        Ok(())
    }
}
