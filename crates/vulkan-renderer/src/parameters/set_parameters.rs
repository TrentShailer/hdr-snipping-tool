use scrgb::ScRGB;
use scrgb_tonemapper::whitepoint::Whitepoint;
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
        self.curve_target = Whitepoint::SdrReferenceWhite;
        self.update_text(vk, glyph_cache)
    }

    pub fn set_parameters(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
        curve_target: Whitepoint,
        whitepoint: ScRGB,
    ) -> Result<(), set_text::Error> {
        self.whitepoint = whitepoint;
        self.curve_target = curve_target;

        self.update_text(vk, glyph_cache)
    }

    fn update_text(
        &mut self,
        vk: &VulkanInstance,
        glyph_cache: &mut GlyphCache,
    ) -> Result<(), set_text::Error> {
        let text = format!(
            "Tonemapping target:\n{}\n{} nits",
            self.curve_target.as_human_readable(),
            self.whitepoint.as_nits_string()
        );

        self.text.set_text(vk, glyph_cache, &text)?;

        Ok(())
    }
}
