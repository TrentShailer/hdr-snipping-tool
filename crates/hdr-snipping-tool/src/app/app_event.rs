use arboard::{Clipboard, ImageData};
use glow::{HasContext, Texture};
use hdr_capture::{Capture, Tonemapper};
use imgui::Textures;
use snafu::{ResultExt, Whatever};

use crate::gui_backend::GuiBackendEvent;

use super::{settings::ImguiSettings, App};

pub enum AppEvent {
    Tonemap,
    RebuildTexture,
    Save,
    Close,
}

impl<T: Tonemapper + ImguiSettings> App<T> {
    pub fn handle_events(
        &mut self,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        let mut queue = self.event_queue.drain(..).collect::<Vec<_>>().into_iter();

        while let Some(event) = queue.next() {
            self.handle_event(event, textures, gl)
                .whatever_context("Failed to handle event")?;
        }

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: AppEvent,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        match event {
            AppEvent::Save => self.save().whatever_context("Failed to save")?,
            AppEvent::Close => self
                .close(textures, gl)
                .whatever_context("Failed to close")?,
            AppEvent::RebuildTexture => self
                .rebuild_texture(textures, gl)
                .whatever_context("Failed to rebuild texture")?,
            AppEvent::Tonemap => self.tonemap(),
        };
        Ok(())
    }

    fn tonemap(&mut self) {
        self.capture.sdr = self.tonemapper.tonemap(&self.capture.hdr);
    }

    fn rebuild_texture(
        &mut self,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        let gl_texture =
            unsafe { gl.create_texture() }.whatever_context("unable to create GL texture")?;
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(gl_texture));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::SRGB as _,
                self.capture.sdr.size.width as _,
                self.capture.sdr.size.height as _,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&self.capture.sdr.data),
            )
        }

        let texture_id = textures.insert(gl_texture);
        self.image_texture_id = Some(texture_id);

        Ok(())
    }

    fn close(
        &mut self,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), Whatever> {
        self.capture = Capture::default();

        self.rebuild_texture(textures, gl)
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
