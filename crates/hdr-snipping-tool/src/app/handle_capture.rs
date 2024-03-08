use error_trace::{ErrorTrace, ResultExt};
use glow::Texture;
use hdr_capture::{Capture, Tonemapper};
use imgui::Textures;
use winit::window::Window;

use super::{selection::SelectionSate, settings::ImguiSettings, window_info::WindowInfo, App};

impl<T: Tonemapper + ImguiSettings> App<T> {
    pub fn handle_capture(
        &mut self,
        window: &Window,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), ErrorTrace> {
        if let Ok((hdr, display_info)) = self.capture_receiver.try_recv() {
            self.tonemapper.reset_settings(&hdr);
            let sdr = self.tonemapper.tonemap(&hdr);
            let mut capture = Capture::new(hdr, sdr);
            capture.selection.size = display_info.size;
            self.capture = capture;

            self.selection_state = SelectionSate::None;

            self.rebuild_texture(textures, gl).track()?;

            let _ = window.request_inner_size(display_info.size);
            window.set_outer_position(display_info.position);

            self.window = WindowInfo::try_from(window).track()?;
        }
        Ok(())
    }
}
