mod imgui_winit_support;
mod utils;

use std::num::NonZeroU32;
use std::time::Instant;

use error_trace::{ErrorTrace, OptionExt, ResultExt};
use glow::HasContext;
use glutin::context::PossiblyCurrentContext;
use glutin::surface::{GlSurface, Surface, WindowSurface};
use imgui::{Context, Textures, Ui};
use imgui_glow_renderer::Renderer;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIcon;
use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::Window;

use self::imgui_winit_support::WinitPlatform;
use self::utils::{create_window, glow_context, imgui_init, init_tray_icon};

#[derive(Debug, PartialEq)]
pub enum GuiBackendEvent {
    ShowWindow,
    HideWindow,
}

pub struct GuiBackend<'a> {
    pub context: PossiblyCurrentContext,
    pub event_loop: &'a mut EventLoop<GuiBackendEvent>,
    pub gl: glow::Context,
    pub imgui_context: Context,
    pub imgui_renderer: Renderer,
    pub surface: Surface<WindowSurface>,
    pub textures: Textures<glow::Texture>,
    pub tray_icon: TrayIcon,
    pub window: Window,
    pub winit_platform: WinitPlatform,
}

/// Initialize the app window and tray icon
pub fn init(event_loop: &mut EventLoop<GuiBackendEvent>) -> Result<GuiBackend, ErrorTrace> {
    let (window, surface, context) = create_window(event_loop, None).track()?;

    let (winit_platform, mut imgui_context) = imgui_init(&window);

    let gl = glow_context(&context);
    unsafe { gl.enable(glow::FRAMEBUFFER_SRGB) };

    let mut textures = imgui::Textures::<glow::Texture>::default();

    let imgui_renderer =
        Renderer::initialize(&gl, &mut imgui_context, &mut textures, false).track()?;

    let tray_icon = init_tray_icon().track()?;
    tray_icon.set_visible(true).track()?;

    Ok(GuiBackend {
        context,
        event_loop,
        gl,
        imgui_context,
        imgui_renderer,
        surface,
        textures,
        tray_icon,
        window,
        winit_platform,
    })
}

impl<'a> GuiBackend<'a> {
    pub fn run<F>(self, mut run_ui: F) -> Result<(), ErrorTrace>
    where
        F: FnMut(&mut Ui, &Window, &mut Textures<glow::Texture>, &glow::Context) + 'static,
    {
        let GuiBackend {
            context,
            event_loop,
            gl,
            mut imgui_context,
            mut imgui_renderer,
            surface,
            mut textures,
            window,
            mut winit_platform,
            ..
        } = self;

        let mut last_frame = Instant::now();
        event_loop
            .run_on_demand(|event, window_target| {
                if let Err(e) = (|| {
                    if !window.is_visible().track()? {
                        window_target.set_control_flow(ControlFlow::Wait);
                    } else {
                        window_target.set_control_flow(ControlFlow::Poll);
                    }

                    if let Ok(tray_event) = MenuEvent::receiver().try_recv() {
                        match tray_event.id.0.as_str() {
                            "0" => window_target.exit(),
                            "1" => window_target.exit(),
                            _ => {}
                        };
                    }

                    match &event {
                        Event::NewEvents(_) => {
                            let now = Instant::now();
                            imgui_context
                                .io_mut()
                                .update_delta_time(now.duration_since(last_frame));
                            last_frame = now;
                        }
                        winit::event::Event::AboutToWait => {
                            winit_platform
                                .prepare_frame(imgui_context.io_mut(), &window)
                                .track()?;

                            window.request_redraw();
                        }
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::RedrawRequested,
                            ..
                        } => {
                            unsafe { gl.clear(glow::COLOR_BUFFER_BIT) };

                            let ui = imgui_context.frame();
                            run_ui(ui, &window, &mut textures, &gl);

                            winit_platform.prepare_render(ui, &window);
                            let draw_data = imgui_context.render();
                            imgui_renderer.render(&gl, &textures, draw_data).track()?;

                            surface.swap_buffers(&context).track()?;
                        }
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::Resized(new_size),
                            ..
                        } => {
                            if new_size.width > 0 && new_size.height > 0 {
                                surface.resize(
                                    &context,
                                    NonZeroU32::new(new_size.width).track()?,
                                    NonZeroU32::new(new_size.height).track()?,
                                );
                            }
                            winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
                        }
                        winit::event::Event::WindowEvent {
                            event: winit::event::WindowEvent::CloseRequested,
                            ..
                        } => {
                            window_target.exit();
                        }
                        winit::event::Event::LoopExiting => {
                            imgui_renderer.destroy(&gl);
                        }
                        Event::UserEvent(v) => match v {
                            GuiBackendEvent::ShowWindow => {
                                window.set_visible(true);
                                window.focus_window();
                            }
                            GuiBackendEvent::HideWindow => {
                                window.set_visible(false);
                                imgui_context
                                    .io_mut()
                                    .add_mouse_button_event(imgui::MouseButton::Left, false);
                            }
                        },
                        event => {
                            winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
                        }
                    }
                    Ok::<(), ErrorTrace>(())
                })() {
                    log::error!("{}", e.to_string());
                }
            })
            .track()?;

        Ok(())
    }
}