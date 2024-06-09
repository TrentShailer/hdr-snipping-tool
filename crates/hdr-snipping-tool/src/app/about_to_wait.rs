use std::time::{Duration, Instant};

use tray_icon::menu::MenuEvent;
use winit::event_loop::ActiveEventLoop;

use super::App;

impl App {
    pub(super) fn handle_about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let app = match self.app.as_ref() {
            Some(app) => app,
            None => return,
        };

        if let Ok(tray_event) = MenuEvent::receiver().try_recv() {
            if tray_event.id.0.as_str() == "0" {
                event_loop.exit()
            }
        }

        // Request a redraw if it has been more than x ms since last frame
        if Instant::now().duration_since(self.last_frame) < Duration::from_millis(8) {
            return;
        }

        app.window.request_redraw();
    }
}
