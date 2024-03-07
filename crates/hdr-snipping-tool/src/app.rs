mod handle_capture;
mod keybinds;
mod mouse_guides;
mod selection;
mod settings;
mod window_info;

use std::sync::mpsc::Receiver;

use arboard::{Clipboard, ImageData};
use error_trace::{ErrorTrace, ResultExt};
use glow::{HasContext, Texture};
use hdr_capture::{Capture, DisplayInfo, HdrCapture, Tonemapper};
use imgui::{TextureId, Textures, Ui};
use winit::{dpi::LogicalPosition, event_loop::EventLoopProxy, window::Window};

use crate::{gui_backend::GuiBackendEvent, settings::Settings};

use self::{selection::SelectionSate, settings::ImguiSettings, window_info::WindowInfo};

pub struct App<T>
where
    T: Tonemapper + ImguiSettings,
{
    pub capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
    pub event_proxy: EventLoopProxy<GuiBackendEvent>,
    pub capture: Capture,
    pub tonemapper: T,
    pub window: WindowInfo,
    pub image_texture_id: Option<TextureId>,
    pub selection_state: SelectionSate,
    pub selection_start: LogicalPosition<f32>,
    pub settings: Settings,
}

impl<T> App<T>
where
    T: Tonemapper + ImguiSettings,
{
    pub fn new(
        capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
        event_proxy: EventLoopProxy<GuiBackendEvent>,
        settings: Settings,
        tonemapper: T,
    ) -> Self {
        Self {
            capture_receiver,
            event_proxy,
            settings,
            tonemapper,
            image_texture_id: None,
            capture: Capture::default(),
            window: WindowInfo::default(),
            selection_start: LogicalPosition::default(),
            selection_state: SelectionSate::default(),
        }
    }

    pub fn update(
        &mut self,
        ui: &mut Ui,
        window: &Window,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), ErrorTrace> {
        self.handle_capture(window, textures, gl)?;

        // draw image
        let scale = self.window.scale;
        let image_size = self.capture.sdr.size.to_logical(scale);

        if let Some(texture_id) = self.image_texture_id {
            ui.get_background_draw_list()
                .add_image(
                    texture_id,
                    [0.0, 0.0],
                    [image_size.width, image_size.height],
                )
                .build();
        }

        self.handle_keybinds(ui).track()?;
        self.draw_selection(ui);
        self.draw_mouse_guides(ui);
        let settings_bounds = self.draw_settings(ui, textures, gl).track()?;
        self.handle_selection(ui, settings_bounds).track()?;

        Ok(())
    }

    fn tonemap(&mut self) {
        self.capture.sdr = self.tonemapper.tonemap(&self.capture.hdr);
    }

    fn rebuild_texture(
        &mut self,
        textures: &mut Textures<Texture>,
        gl: &glow::Context,
    ) -> Result<(), ErrorTrace> {
        let gl_texture = unsafe { gl.create_texture() }.track()?;
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

    fn close(&mut self) -> Result<(), ErrorTrace> {
        self.capture = Capture::default();
        self.image_texture_id = None;

        self.event_proxy
            .send_event(GuiBackendEvent::HideWindow)
            .track()?;

        Ok(())
    }

    fn save(&mut self) -> Result<(), ErrorTrace> {
        let image = self.capture.save_capture().track()?;

        let mut clipboard = Clipboard::new().track()?;

        clipboard
            .set_image(ImageData {
                width: self.capture.selection.size.width as usize,
                height: self.capture.selection.size.height as usize,
                bytes: std::borrow::Cow::Borrowed(&image.as_raw()),
            })
            .track()?;

        Ok(())
    }
}
