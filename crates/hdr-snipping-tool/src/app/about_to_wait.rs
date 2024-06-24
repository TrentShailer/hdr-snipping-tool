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

        app.window.request_redraw();
    }
}
