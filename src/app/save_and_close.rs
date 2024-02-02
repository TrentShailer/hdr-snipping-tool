use arboard::{Clipboard, ImageData};
use glium::Display;
use imgui::Textures;
use imgui_glium_renderer::Texture;

use crate::{gui::AppEvent, image::Image};

use super::App;

impl App {
    pub fn save_and_close(&mut self, display: &Display, textures: &mut Textures<Texture>) {
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

        self.image = Image::blank();
        self.remake_texture(display, textures).unwrap();
    }
}
