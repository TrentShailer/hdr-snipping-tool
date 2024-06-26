use super::App;

impl App {
    pub fn clear_capture(&mut self) {
        self.capture = None;
        self.scroll = 0.0;

        let app = match self.app.as_mut() {
            Some(v) => v,
            None => return,
        };

        app.window.set_visible(false);
        app.renderer.capture.unload_capture();
    }
}
