use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::event_loop::ActiveEventLoop;

use crate::message_box::display_message;

use super::App;

impl App {
    pub fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return,
        };

        if !app.window.is_visible().unwrap_or(true) {
            return;
        }

        let result = app.renderer.render(
            &app.vulkan_instance,
            app.window.clone(),
            self.selection.as_ltrb(),
            self.mouse_position.into(),
        );

        if let Err(e) = result {
            log::error!("{e}");
            display_message(
                "We encountered an error during rendering.\nMore details are in the logs.",
                MB_ICONERROR,
            );
            event_loop.exit();
        }
    }
}
