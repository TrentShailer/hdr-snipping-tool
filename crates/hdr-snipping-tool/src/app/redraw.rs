use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::event_loop::ActiveEventLoop;

use crate::message_box::display_message;

use super::App;

impl App {
    pub fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut() else { return };

        if !app.window.is_visible().unwrap_or(true) {
            return;
        }

        let selection = self.selection.as_pos_size();

        let result = app.renderer.render(
            &app.vulkan_instance,
            app.window.clone(),
            selection.0.into(),
            selection.1.into(),
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
