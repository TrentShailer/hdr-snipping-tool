use std::rc::Rc;

use arboard::{Clipboard, ImageData};
use glium::{
    texture::RawImage2d,
    uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior},
    Display, Texture2d,
};
use hdr_snipping_tool::Capture;
use imgui_glium_renderer::{Renderer, Texture};
use snafu::{ResultExt, Whatever};

use crate::gui_backend::GuiBackendEvent;

use super::App;

pub enum AppEvent {
    Tonemap,
    RebuildTexture,
    Save,
    Close,
    ReloadGui,
}

impl App {
    pub fn handle_events(
        &mut self,
        display: &Display,
        renderer: &mut Renderer,
    ) -> Result<(), Whatever> {
        let mut queue = self.event_queue.drain(..).collect::<Vec<_>>().into_iter();

        while let Some(event) = queue.next() {
            self.handle_event(event, display, renderer)
                .whatever_context("Failed to handle event")?;
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: AppEvent,
        display: &Display,
        renderer: &mut Renderer,
    ) -> Result<(), Whatever> {
        match event {
            AppEvent::Save => self.save().whatever_context("Failed to save")?,
            AppEvent::Close => self
                .close(display, renderer)
                .whatever_context("Failed to close")?,
            AppEvent::RebuildTexture => self
                .rebuild_texture(display, renderer)
                .whatever_context("Failed to rebuild texture")?,
            AppEvent::Tonemap => self.tone_map(),
            AppEvent::ReloadGui => self
                .event_proxy
                .send_event(GuiBackendEvent::ReloadGui)
                .whatever_context("Failed to send reload event to gui backend")?,
        };
        Ok(())
    }

    fn tone_map(&mut self) {
        self.capture.sdr = self.capture.tone_mapper.tonemap(&self.capture.hdr);
    }

    fn rebuild_texture(
        &mut self,
        display: &Display,
        renderer: &mut Renderer,
    ) -> Result<(), Whatever> {
        let raw = RawImage2d {
            data: std::borrow::Cow::Borrowed(&self.capture.sdr.data),
            width: self.capture.sdr.size.width,
            height: self.capture.sdr.size.height,
            format: glium::texture::ClientFormat::U8U8U8U8,
        };

        let gl_texture =
            Texture2d::new(display, raw).whatever_context("Failed to make texture2d")?;

        let texture = Texture {
            texture: Rc::new(gl_texture),
            sampler: SamplerBehavior {
                magnify_filter: MagnifySamplerFilter::Linear,
                minify_filter: MinifySamplerFilter::Linear,
                ..Default::default()
            },
        };

        let texture_id = renderer.textures().insert(texture);
        self.image_texture_id = Some(texture_id);

        Ok(())
    }

    fn close(&mut self, display: &Display, renderer: &mut Renderer) -> Result<(), Whatever> {
        self.capture = Capture::default();
        self.rebuild_texture(display, renderer)
            .whatever_context("Failed to rebuild texture")?;
        self.event_proxy
            .send_event(GuiBackendEvent::HideWindow)
            .whatever_context("Failed to send event to event proxy")?;

        Ok(())
    }

    fn save(&mut self) -> Result<(), Whatever> {
        let image = self
            .capture
            .save_capture()
            .whatever_context("Failed to save capture")?;

        let mut clipboard = Clipboard::new().whatever_context("Failed to open clipboard")?;

        clipboard
            .set_image(ImageData {
                width: self.capture.selection.size.width as usize,
                height: self.capture.selection.size.height as usize,
                bytes: std::borrow::Cow::Borrowed(&image.as_raw()),
            })
            .whatever_context("Failed to set image in clipboard")?;

        Ok(())
    }
}
