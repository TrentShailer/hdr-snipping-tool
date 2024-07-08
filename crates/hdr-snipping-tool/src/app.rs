mod clear_capture;
mod init;
mod keyboard_input;
mod mouse_wheel;
mod redraw;
mod save;
mod take_capture;
mod tray_icon;
mod update_tonemapper_settings;

use std::{process::Command, sync::Arc};

use ::tray_icon::{menu::MenuEvent, TrayIcon};
use vulkan_instance::{texture::Texture, VulkanInstance};
use vulkan_renderer::renderer::Renderer;
use vulkan_tonemapper::Tonemapper;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use windows_capture_provider::WindowsCaptureProvider;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

use crate::{
    message_box::display_message, project_directory, selection::Selection, settings::Settings,
};

pub struct ActiveApp {
    pub window_id: WindowId,
    pub window: Arc<Window>,
    pub _tray_icon: TrayIcon,
    pub vulkan_instance: VulkanInstance,
    pub renderer: Renderer,
}

pub struct ActiveCapture {
    pub tonemapper: Tonemapper,
    pub texture: Arc<Texture>,
}

pub struct App {
    pub app: Option<ActiveApp>,
    pub capture: Option<ActiveCapture>,
    pub capture_provider: WindowsCaptureProvider,
    pub settings: Settings,
    pub mouse_position: PhysicalPosition<u32>,
    pub selection: Selection,
    pub scroll: f32,
    pub keyboard_modifiers: ModifiersState,
}

impl App {
    pub fn new(capture_provider: WindowsCaptureProvider, settings: Settings) -> Self {
        Self {
            capture_provider,
            settings,
            app: None,
            capture: None,
            mouse_position: PhysicalPosition::default(),
            selection: Selection::default(),
            scroll: 0.0,
            keyboard_modifiers: ModifiersState::default(),
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let init_result = self.init(event_loop);

        if let Err(e) = init_result {
            log::error!("{e}");
            let message = match e {
                init::Error::CreateWindow(_) =>
                    "We encountered an error while creating the window.\nMore details are in the logs.",
                init::Error::Icon(_) =>
                    "We encountered an error while getting the app icon.\nMore details are in the logs.",
                init::Error::TrayIcon(_) =>
                    "We encountered an error while creating the tray icon.\nMore details are in the logs.",
                init::Error::TrayIconVisible(_) =>
                    "We encountered an error while changing the tray icon visibility.\nMore details are in the logs.",
                init::Error::VulkanInstance(_) =>
                    "We encountered an error while creating the Vulkan instance.\nMore details are in the logs.",
                init::Error::Renderer(_) =>
                    "We encountered an error while creating the renderer.\nMore details are in the logs.",
            };
            display_message(message, MB_ICONERROR);
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        let result = self.take_capture();

        if let Err(e) = result {
            log::error!("{e}");
            let message = match e {
                take_capture::Error::CaptureProvider(_) => "We encountered an error while getting the capture from windows.\nMore details are in the logs.",
                take_capture::Error::Texture(_) => "We encountered an error while creating the texture.\nMore details are in the logs.",
                take_capture::Error::Tonemapper(_) => "We encountered an error while creating the tonemapper.\nMore details are in the logs.",
                take_capture::Error::UpdateRenderer(_) => "We encountered an error while updaing the renderer.\nMore details are in the logs.",
                take_capture::Error::Tonemap(_) => "We encountered an error while tonemapping.\nMore details are in the logs.",
                take_capture::Error::LoadCapture(_) => "We encountered an error while loading the capture into the renderer.\nMore details are in the logs.",
            };

            display_message(message, MB_ICONERROR);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_ref() else { return };

        if let Ok(tray_event) = MenuEvent::receiver().try_recv() {
            match tray_event.id.0.as_str() {
                "0" => {
                    if let Err(e) = Command::new("explorer").arg(project_directory()).spawn() {
                        log::error!("{e}");
                        display_message("We encountered an error while opening file explor\nMore details are in the logs.", MB_ICONERROR);
                        event_loop.exit();
                    }
                }
                "1" => event_loop.exit(),
                _ => {}
            }
        }

        app.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else { return };

        if event == WindowEvent::Destroyed && app.window_id == window_id {
            self.clear_capture();
            self.app = None;
            event_loop.exit();
            return;
        }

        if self.capture.as_ref().is_none() {
            return;
        }

        match event {
            WindowEvent::Resized(_new_size) => {
                app.renderer.recreate_swapchain = true;
                app.window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                self.clear_capture();
                self.app = None;
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            //
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => self.keyboard_input(event, event_loop),
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => self.mouse_wheel(delta, event_loop),
            WindowEvent::ModifiersChanged(modifiers) => self.keyboard_modifiers = modifiers.state(),
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if button != MouseButton::Left {
                    return;
                }

                match state {
                    winit::event::ElementState::Pressed => {
                        self.selection
                            .mouse_pressed(self.mouse_position, app.window.inner_size());
                    }
                    winit::event::ElementState::Released => {
                        let should_save = self.selection.mouse_released();
                        if should_save {
                            let result = self.save_capture();

                            if let Err(e) = result {
                                log::error!("{e}");
                                display_message("We encountered an error while saving the capture.\nMore details are in the logs.", MB_ICONERROR);
                                event_loop.exit();
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.cast();
                self.selection
                    .mouse_moved(self.mouse_position, app.window.inner_size());
            }
            _ => (),
        }
    }
}
