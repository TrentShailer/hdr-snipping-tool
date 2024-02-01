use arboard::{Clipboard, ImageData};

use crate::gui::AppEvent;

use super::App;

impl App {
    pub fn save_and_close(&mut self) {
        self.proxy.send_event(AppEvent::Hide).unwrap();

        let image = self.image.save();
        let mut clipboard = Clipboard::new().unwrap();
        clipboard
            .set_image(ImageData {
                width: image.width() as usize,
                height: image.height() as usize,
                bytes: std::borrow::Cow::Borrowed(&image.as_raw()),
            })
            .unwrap();
    }
}
