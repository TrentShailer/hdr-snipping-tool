use std::{error::Error, rc::Rc, sync::mpsc::Receiver};

use arboard::{Clipboard, ImageData};
use glium::{
    backend::Facade,
    glutin::event_loop::EventLoopProxy,
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
        }

        if ui.is_key_down(imgui::Key::Escape) {
            self.proxy.send_event(AppEvent::Hide).unwrap();
            return;
        }

        if ui.is_key_down(imgui::Key::Enter) {
            let image = self.image.save();
            let mut clipboard = Clipboard::new().unwrap();
            clipboard
                .set_image(ImageData {
                    width: image.width() as usize,
                    height: image.height() as usize,
                    bytes: std::borrow::Cow::Borrowed(&image.as_raw()),
                })
                .unwrap();
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

        let selection_rect = self.image.get_selection_rect();
        ui.get_foreground_draw_list()
            .add_rect(selection_rect[0], selection_rect[1], 0xff_5e_e0_f6)
            .thickness(1.0)
            .build();

        let (size, pos) = ui
            .window("Settings")
            .always_auto_resize(true)
            .collapsible(false)
            .build(|| {
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

                if ui.button_with_size("Save and Close", [250.0, 25.0]) {
                    let image = self.image.save();
                    let mut clipboard = Clipboard::new().unwrap();
                    clipboard
                        .set_image(ImageData {
                            width: image.width() as usize,
                            height: image.height() as usize,
                            bytes: std::borrow::Cow::Borrowed(&image.as_raw()),
                        })
                        .unwrap();

                    self.proxy.send_event(AppEvent::Hide).unwrap();
                }

                (ui.window_size(), ui.window_pos())
            })
            .unwrap();

        if ui.is_mouse_down(imgui::MouseButton::Left)
            && !self.selecting
            && !is_inside(ui.io().mouse_pos, pos, size)
        {
            self.selecting = true;
            self.selection_start = ui.io().mouse_pos;
        }

        if ui.is_mouse_dragging(imgui::MouseButton::Left) && self.selecting {
            let cur_pos = ui.io().mouse_pos;
            let start_pos = self.selection_start;

            // find the leftmost point
            let left = if cur_pos[0] < start_pos[0] {
                cur_pos[0]
            } else {
                start_pos[0]
            };

            let right = if cur_pos[0] > start_pos[0] {
                cur_pos[0]
            } else {
                start_pos[0]
            };

            let top = if cur_pos[1] < start_pos[1] {
                cur_pos[1]
            } else {
                start_pos[1]
            };

            let bottom = if cur_pos[1] > start_pos[1] {
                cur_pos[1]
            } else {
                start_pos[1]
            };

            self.image.selection_pos = [left as u32, top as u32];
            self.image.selection_size = [(right - left) as u32, (bottom - top) as u32];
        }

        if ui.is_mouse_released(imgui::MouseButton::Left) && self.selecting {
            self.selecting = false;
        }

        ui.get_foreground_draw_list()
            .add_line(
                [0.0, ui.io().mouse_pos[1]],
                [
                    display.gl_window().window().inner_size().width as f32,
                    ui.io().mouse_pos[1],
                ],
                0x20_80_80_80,
            )
            .build();

        ui.get_foreground_draw_list()
            .add_line(
                [ui.io().mouse_pos[0], 0.0],
                [
                    ui.io().mouse_pos[0],
                    display.gl_window().window().inner_size().height as f32,
                ],
                0x20_80_80_80,
            )
            .build();
    }
}

fn is_inside(point: [f32; 2], pos: [f32; 2], size: [f32; 2]) -> bool {
    point[0] >= pos[0]
        && point[0] <= pos[0] + size[0]
        && point[1] >= pos[1]
        && point[1] <= pos[1] + size[1]
}
