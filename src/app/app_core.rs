use std::{error::Error, rc::Rc, sync::mpsc::Receiver};

use glium::{
    backend::Facade,
    glutin::event_loop::EventLoopProxy,
    texture::RawImage2d,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Display, Texture2d,
};
use imgui::{TextureId, Textures, Ui};
use imgui_glium_renderer::Texture;

use crate::{capture::DisplayInfo, gui::AppEvent, image::Image};

pub struct App {
    pub image: Image,
    pub proxy: EventLoopProxy<AppEvent>,
    pub receiver: Receiver<(Image, DisplayInfo)>,
    pub texture_id: Option<TextureId>,
    pub selecting: bool,
    pub selection_start: [f32; 2],
}

impl App {
    pub fn new(receiver: Receiver<(Image, DisplayInfo)>, proxy: EventLoopProxy<AppEvent>) -> Self {
        Self {
            image: Image::blank(),
            proxy,
            receiver,
            texture_id: None,
            selecting: false,
            selection_start: [0.0, 0.0],
        }
    }

    pub fn remake_texture(
        &mut self,
        gl_ctx: &dyn Facade,
        textures: &mut Textures<Texture>,
    ) -> Result<(), Box<dyn Error>> {
        let raw = RawImage2d {
            data: std::borrow::Cow::Borrowed(&self.image.current),
            width: self.image.width,
            height: self.image.height,
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
        // receive image
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
        }

        self.handle_keybinds(ui);

        // draw image
        ui.get_background_draw_list()
            .add_image(
                self.texture_id.unwrap(),
                [0.0, 0.0],
                [self.image.width as f32, self.image.height as f32],
            )
            .build();

        self.draw_selection(ui, display);

        let (size, pos) = self.draw_settings(ui, display, textures);

        self.handle_selection(ui, display, pos, size);

        self.draw_mouse_guides(ui, display);
    }
}
