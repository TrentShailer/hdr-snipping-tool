use std::{error::Error, rc::Rc, sync::mpsc::Receiver};

use glium::{
    backend::Facade,
    glutin::event_loop::EventLoopProxy,
    texture::{RawImage1d, RawImage2d},
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Texture2d,
};
use imgui::{TextureId, Textures, Ui};
use imgui_glium_renderer::Texture;

use crate::{image::Image, support::AppEvent};

pub struct App {
    pub image: Image,
    pub proxy: EventLoopProxy<AppEvent>,
    pub receiver: Receiver<Image>,
    pub texture_id: Option<TextureId>,
}

impl App {
    pub fn new(receiver: Receiver<Image>, proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            image: Image::blank(),
            proxy,
            receiver,
            texture_id: None,
        }
    }

    pub fn remake_texture(
        &mut self,
        gl_ctx: &dyn Facade,
        textures: &mut Textures<Texture>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(id) = self.texture_id {
            textures.remove(id);
        }

        println!("{}", self.image.width);
        println!("{}", self.image.height);
        println!("{}", self.image.current.len());

        let image = self.image.current.to_vec();

        let raw = RawImage2d {
            data: std::borrow::Cow::Owned(image),
            width: self.image.width as u32,
            height: self.image.height as u32,
            format: glium::texture::ClientFormat::U8U8U8U8,
        };
        let gl_texture = Texture2d::new(gl_ctx, raw)?;
        let texture = Texture {
            texture: Rc::new(gl_texture),
            sampler: SamplerBehavior {
                magnify_filter: MagnifySamplerFilter::Linear,
                minify_filter: MinifySamplerFilter::Linear,
                ..Default::default()
            },
        };
        let texture_id = textures.insert(texture);
        self.texture_id = Some(texture_id);

        Ok(())
    }

    pub fn render(&mut self, ui: &mut Ui, gl_ctx: &dyn Facade, textures: &mut Textures<Texture>) {
        if let Ok(image) = self.receiver.try_recv() {
            println!("Got image");
            self.image = image;
            self.remake_texture(gl_ctx, textures).unwrap();

            return;
        }

        if ui.is_key_down(imgui::Key::Escape) {
            self.proxy.send_event(AppEvent::Hide).unwrap();
            return;
        }

        ui.get_background_draw_list()
            .add_image(
                self.texture_id.unwrap(),
                [0.0, 0.0],
                [self.image.width as f32, self.image.height as f32],
            )
            .build();
        // draw image

        ui.window("Settings").always_auto_resize(true).build(|| {
            if ui.slider("Gamma", 0.01, 1.0, &mut self.image.gamma) {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
            }

            if ui.slider("Alpha", 0.01, 10.0, &mut self.image.alpha) {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
            }

            if ui.button("Auto Alpha") {
                self.image.alpha = self.image.calculate_alpha();
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
            }
        });
    }
}
