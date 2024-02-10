mod app_event;
mod handle_capture;
mod keybinds;
mod mouse_guides;
mod selection;
mod settings;
mod window_info;

use std::{collections::VecDeque, sync::mpsc::Receiver};

use glium::{glutin::event_loop::EventLoopProxy, Display};
use hdr_snipping_tool::{Capture, DisplayInfo, HdrCapture, LogicalPosition};
use imgui::{TextureId, Ui};
use imgui_glium_renderer::Renderer;
use snafu::{ResultExt, Whatever};

use crate::gui_backend::GuiBackendEvent;

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
}

impl App {
    pub fn new(
        capture_receiver: Receiver<(HdrCapture, DisplayInfo)>,
        event_proxy: EventLoopProxy<GuiBackendEvent>,
    ) -> Self {
        Self {
            capture_receiver,
            event_proxy,
            event_queue: VecDeque::default(),
            capture: Capture::default(),
            window: WindowInfo::default(),
            selection_start: LogicalPosition::default(),
            selection_state: SelectionSate::default(),
            image_texture_id: None,
        }
    }

    pub fn update(
        &mut self,
        ui: &mut Ui,
        display: &Display,
        renderer: &mut Renderer,
    ) -> Result<(), Whatever> {
        self.handle_capture(display, renderer)
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

        self.handle_events(display, renderer)
            .whatever_context("Failed to handle events")?;

        Ok(())
    }
}
