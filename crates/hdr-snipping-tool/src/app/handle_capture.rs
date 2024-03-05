use gamma_compression_tonemapper::GammaCompressionTonemapper;
use glow::Texture;
use hdr_capture::Capture;
use imgui::Textures;
use snafu::{ResultExt, Whatever};
use winit::window::Window;

use super::{app_event::AppEvent, selection::SelectionSate, window_info::WindowInfo, App};

impl App {
    pub fn handle_capture(
        &mut self,
        window: &Window,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        if let Ok((hdr, display_info)) = self.capture_receiver.try_recv() {
            let tone_mapper = GammaCompressionTonemapper::new(&hdr, self.settings.default_gamma);
            self.capture = Capture::new(hdr, Box::new(tone_mapper));
            self.selection_state = SelectionSate::None;

            self.event_queue.push_back(AppEvent::RebuildTexture);
            self.handle_events(window, textures, gl)
                .whatever_context("Failed to handle events")?;

            let _ = window.request_inner_size(display_info.size);
            window.set_outer_position(display_info.position);

            self.window =
                WindowInfo::try_from(window).whatever_context("Failed to get window info")?;
        }
        Ok(())
    }
}
