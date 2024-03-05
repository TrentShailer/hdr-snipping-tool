mod app_event;
mod handle_capture;
mod keybinds;
mod mouse_guides;
mod selection;
mod settings;
mod window_info;

use std::{collections::VecDeque, sync::mpsc::Receiver};

use glow::Texture;
use hdr_capture::{Capture, DisplayInfo, HdrCapture, Tonemapper};
use imgui::{TextureId, Textures, Ui};
use snafu::{ResultExt, Whatever};
use winit::{dpi::LogicalPosition, event_loop::EventLoopProxy, window::Window};

use crate::{gui_backend::GuiBackendEvent, settings::Settings};

use self::{
    app_event::AppEvent, selection::SelectionSate, settings::ImguiSettings, window_info::WindowInfo,
};

pub struct App<T>
where
    T: Tonemapper + ImguiSettings,
{
    pub capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
    pub event_proxy: EventLoopProxy<GuiBackendEvent>,
    pub capture: Capture,
    pub tonemapper: T,
    pub window: WindowInfo,
    pub event_queue: VecDeque<AppEvent>,
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
            event_queue: VecDeque::default(),
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
    ) -> Result<(), Whatever> {
        self.handle_capture(window, textures, gl)
            .whatever_context("Failed to receive capture")?;

        self.handle_keybinds(ui);

        // draw image
        let scale = self.window.scale;
        let image_size = self.capture.sdr.size.to_logical(scale);
        ui.get_background_draw_list()
            .add_image(
                self.image_texture_id.unwrap(),
                [0.0, 0.0],
                [image_size.width, image_size.height],
            )
            .build();

        self.draw_selection(ui);
        self.draw_mouse_guides(ui);
        let settings_bounds = self.draw_settings(ui);
        self.handle_selection(ui, settings_bounds);

        self.handle_events(textures, gl)
            .whatever_context("Failed to handle events")?;

        Ok(())
    }
}
