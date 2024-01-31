use std::{error::Error, rc::Rc, sync::mpsc::Receiver};

use glium::{
    backend::Facade,
    glutin::{
        dpi::{LogicalSize, PhysicalPosition},
        event_loop::EventLoopProxy,
    },
    texture::RawImage2d,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Display, Texture2d,
};
use imgui::{TextureId, Textures, Ui};
use imgui_glium_renderer::Texture;

use crate::{display::DisplayInfo, image::Image, support::AppEvent};

pub struct App {
    pub image: Image,
    pub proxy: EventLoopProxy<AppEvent>,
    pub receiver: Receiver<(Image, DisplayInfo)>,
    pub texture_id: Option<TextureId>,
}

impl App {
    pub fn new(receiver: Receiver<(Image, DisplayInfo)>, proxy: EventLoopProxy<AppEvent>) -> Self {
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
        /* if let Some(id) = self.texture_id {
            textures.remove(id);
        } */

        let raw = RawImage2d {
            data: std::borrow::Cow::Borrowed(&self.image.current),
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

    pub fn render(&mut self, ui: &mut Ui, display: &Display, textures: &mut Textures<Texture>) {
        if let Ok((image, image_display)) = self.receiver.try_recv() {
            self.image = image;

            self.remake_texture(display.get_context(), textures)
                .unwrap();

            display
                .gl_window()
                .window()
                .set_inner_size(image_display.get_size());

            display
                .gl_window()
                .window()
                .set_outer_position(image_display.get_position());
            // return;
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

        ui.get_foreground_draw_list()
            .add_polyline(
                vec![
                    [0.0, 0.0],
                    [0.0, self.image.height as f32],
                    [self.image.width as f32, self.image.height as f32],
                    [self.image.width as f32, 0.0],
                    [0.0, 0.0],
                ],
                0xff_5e_e0_f6,
            )
            .thickness(5.0)
            .build();

        ui.window("Settings").always_auto_resize(true).build(|| {
            if ui.slider("Gamma", 0.01, 1.0, &mut self.image.gamma) {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
            }

            if ui.slider("Alpha", 0.01, 10.0, &mut self.image.alpha) {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
            }

            if ui.button_with_size("Auto Alpha", [250.0, 25.0]) {
                self.image.alpha = self.image.calculate_alpha();
            }
            if ui.button_with_size("Apply", [250.0, 25.0]) {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
                self.remake_texture(display.get_context(), textures)
                    .unwrap();
            }
        });
    }
}
