mod app_event;
mod handle_capture;
mod keybinds;
mod mouse_guides;
mod selection;
mod settings;
mod window_info;

use std::{collections::VecDeque, sync::mpsc::Receiver};

use gamma_compression_tonemapper::GammaCompressionTonemapper;
use glow::Texture;
use hdr_capture::{Capture, DisplayInfo, HdrCapture};
use imgui::{TextureId, Textures, Ui};
use snafu::{ResultExt, Whatever};
use winit::{dpi::LogicalPosition, event_loop::EventLoopProxy, window::Window};

use crate::{gui_backend::GuiBackendEvent, settings::Settings};

use self::{app_event::AppEvent, selection::SelectionSate, window_info::WindowInfo};

pub struct App {
    pub capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
    pub event_proxy: EventLoopProxy<GuiBackendEvent>,
    pub capture: Capture,
    pub window: WindowInfo,
    pub event_queue: VecDeque<AppEvent>,
    pub image_texture_id: Option<TextureId>,
    pub selection_state: SelectionSate,
    pub selection_start: LogicalPosition<f32>,
    pub settings: Settings,
}

impl App {
    pub fn new(
        capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
        event_proxy: EventLoopProxy<GuiBackendEvent>,
        settings: Settings,
    ) -> Self {
        Self {
            capture_receiver,
            event_proxy,
            settings,
            event_queue: VecDeque::default(),
            capture: Capture::new(
                HdrCapture::default(),
                Box::new(GammaCompressionTonemapper::default()),
            ),
            window: WindowInfo::default(),
            selection_start: LogicalPosition::default(),
            selection_state: SelectionSate::default(),
            image_texture_id: None,
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
        // self.draw_mouse_guides(ui);
        let settings_bounds = self.draw_settings(ui);
        self.handle_selection(ui, settings_bounds);

        self.handle_events(window, textures, gl)
            .whatever_context("Failed to handle events")?;

        Ok(())
    }
}
