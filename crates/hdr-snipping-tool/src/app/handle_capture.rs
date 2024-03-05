use glow::Texture;
use hdr_capture::{Capture, Tonemapper};
use imgui::Textures;
use snafu::{ResultExt, Whatever};
use winit::window::Window;

use super::{
    app_event::AppEvent, selection::SelectionSate, settings::ImguiSettings,
    window_info::WindowInfo, App,
};

impl<T: Tonemapper + ImguiSettings> App<T> {
    pub fn handle_capture(
        &mut self,
        window: &Window,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        if let Ok((hdr, display_info)) = self.capture_receiver.try_recv() {
            self.tonemapper.reset_settings(&hdr);
            let sdr = self.tonemapper.tonemap(&hdr);
            self.capture = Capture::new(hdr, sdr);

            self.selection_state = SelectionSate::None;

            self.event_queue.push_back(AppEvent::RebuildTexture);
            self.handle_events(textures, gl)
                .whatever_context("Failed to handle events")?;

            let _ = window.request_inner_size(display_info.size);
            window.set_outer_position(display_info.position);

            self.window =
                WindowInfo::try_from(window).whatever_context("Failed to get window info")?;
        }
        Ok(())
    }
}
