use super::App;

impl App {
    pub fn clear_capture(&mut self) {
        self.capture = None;
        self.scroll = 0.0;

        let Some(app) = self.app.as_mut() else { return };

        app.window.set_visible(false);
        app.renderer.capture.unload_capture();
    }
}
