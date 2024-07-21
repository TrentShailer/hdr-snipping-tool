use thiserror::Error;

use crate::windows_helpers::foreground_window::set_foreground_window;

use super::ActiveApp;

impl ActiveApp {
    pub fn clear_capture(&mut self) -> Result<(), Error> {
        self.scroll = 0.0;
        self.renderer.capture.unload_capture();
        self.renderer
            .parameters
            .clear_parameters(&self.vk, &mut self.renderer.glyph_cache)?;

        if let Some(capture) = self.active_capture.as_ref() {
            let selection = capture.selection.as_pos_size();
            self.renderer.render(
                &self.vk,
                self.window.clone(),
                selection.0.into(),
                selection.1.into(),
                self.mouse_position.into(),
            )?;
            set_foreground_window(capture.formerly_focused_window);
        }

        self.active_capture = None;
        self.window.set_visible(false);

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to clear renderer:\n{0}")]
    Render(#[from] vulkan_renderer::renderer::render::Error),

    #[error("Failed to reset renderer text:\n{0}")]
    Text(#[from] vulkan_renderer::text::set_text::Error),
}
