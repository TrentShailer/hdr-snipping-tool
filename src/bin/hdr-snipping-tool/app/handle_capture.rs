use glium::Display;
use hdr_snipping_tool::{Capture, GammaCompressionTonemapper};
use imgui_glium_renderer::Renderer;
use snafu::{ResultExt, Whatever};

use super::{app_event::AppEvent, selection::SelectionSate, window_info::WindowInfo, App};

impl App {
    pub fn handle_capture(
        &mut self,
        display: &Display,
        renderer: &mut Renderer,
    ) -> Result<(), Whatever> {
        if let Ok((hdr, display_info)) = self.capture_receiver.try_recv() {
            let tone_mapper = GammaCompressionTonemapper::new(&hdr, self.settings.default_gamma);
            self.capture = Capture::new(hdr, Box::new(tone_mapper));
            self.selection_state = SelectionSate::None;

            self.event_queue.push_back(AppEvent::RebuildTexture);
            self.handle_events(display, renderer)
                .whatever_context("Failed to handle events")?;

            display
                .gl_window()
                .window()
                .set_inner_size(display_info.size);

            display
                .gl_window()
                .window()
                .set_outer_position(display_info.position);

            self.window =
                WindowInfo::try_from(display).whatever_context("Failed to get window info")?;
        }
        Ok(())
    }
}
