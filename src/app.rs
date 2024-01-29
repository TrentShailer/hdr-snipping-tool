use std::sync::mpsc::Receiver;

use egui::{Context, Frame, Key, Pos2, Slider, Vec2};

use crate::image::Image;

pub struct App {
    pub image: Image,
    pub texture: Option<egui::TextureHandle>,
}

impl App {
    pub fn new(image: Image) -> Self {
        Self {
            image,
            texture: None,
        }
    }

    fn rebuild_texture(&mut self, ctx: &Context) {
        ctx.forget_all_images();
        let handle = ctx.load_texture(
            "screenshot",
            self.image.get_color_image(),
            Default::default(),
        );
        self.texture = Some(handle);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
                    ctx.forget_all_images();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ctx.input(|i| i.key_pressed(Key::Enter)) {
                    // TODO copy to clipboard
                    self.image.save();
                    ctx.forget_all_images();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                if let Some(texture) = &self.texture {
                    let texture = egui::load::SizedTexture::new(
                        texture.id(),
                        egui::vec2(self.image.width as f32, self.image.height as f32),
                    );
                    ui.image(texture);
                } else {
                    self.rebuild_texture(ctx);
                }
            });

        egui::Window::new("Settings").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.image.gamma, 0.1..=1.0).text("Gamma value"));
            ui.add(Slider::new(&mut self.image.alpha, 0.1..=2.0).text("Alpha value"));
            if ui.button("Auto calculate alpha").clicked() {
                self.image.alpha = self.image.calculate_alpha();
            }
            if ui.button("Apply").clicked() {
                self.image.current =
                    Image::compress_gamma(&self.image.raw, self.image.alpha, self.image.gamma);
                self.rebuild_texture(ctx);
            }
        });
    }
}
