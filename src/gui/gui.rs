use anyhow::Context as ErrorContext;
use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontSource, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;

use super::AppEvent;

pub struct Gui {
    pub event_loop: EventLoop<AppEvent>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
}

pub fn init(window_builder: WindowBuilder) -> anyhow::Result<Gui> {
    let event_loop = EventLoopBuilder::with_user_event().build();
    let context = glutin::ContextBuilder::new().with_vsync(true);

    let display = Display::new(window_builder, context, &event_loop)
        .context("Failed to initialize display.")?;

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();

        let dpi_mode = HiDpiMode::Default;

        platform.attach_window(imgui.io_mut(), window, dpi_mode);
    }

    let font_size = 13.0;

    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("../fonts/Inter-Regular.ttf"),
        size_pixels: font_size,
        config: Some(FontConfig {
            rasterizer_multiply: 1.5,
            oversample_h: 4,
            oversample_v: 4,
            ..FontConfig::default()
        }),
    }]);

    let renderer =
        Renderer::init(&mut imgui, &display).context("Failed to initialize renderer.")?;

    Ok(Gui {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
    })
}

impl Gui {
    pub fn main_loop<F: FnMut(&mut bool, &Display, &mut Renderer, &mut Ui) + 'static>(
        self,
        mut run_ui: F,
    ) {
        let Gui {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;
        let mut last_frame = Instant::now();

        let mut just_requested_redraw = false;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Wait;

            if let Event::UserEvent(v) = &event {
                if v == &AppEvent::Redraw {
                    if just_requested_redraw {
                        just_requested_redraw = false;
                        return;
                    } else {
                        just_requested_redraw = true;
                    }
                }
            }

            match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => {
                    let gl_window = display.gl_window();
                    if let Err(e) = platform
                        .prepare_frame(imgui.io_mut(), gl_window.window())
                        .context("Failed to prepare frame")
                    {
                        log::error!("{:?}", e);
                        *control_flow = ControlFlow::Exit;
                    };
                    gl_window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    if let Err(e) = redraw(
                        &mut imgui,
                        &mut run_ui,
                        &display,
                        &mut renderer,
                        control_flow,
                        &mut platform,
                    )
                    .context("Failed to redraw.")
                    {
                        log::error!("{:?}", e);
                        *control_flow = ControlFlow::Exit;
                    };
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::UserEvent(v) => match v {
                    AppEvent::Show => {
                        display.gl_window().window().set_visible(true);
                        display.gl_window().window().focus_window();
                    }
                    AppEvent::Hide => {
                        display.gl_window().window().set_visible(false);
                    }
                    AppEvent::Redraw => {
                        if let Err(e) = redraw(
                            &mut imgui,
                            &mut run_ui,
                            &display,
                            &mut renderer,
                            control_flow,
                            &mut platform,
                        )
                        .context("Failed to redraw.")
                        {
                            log::error!("{:?}", e);
                            *control_flow = ControlFlow::Exit;
                        };
                    }
                },
                event => {
                    let gl_window = display.gl_window();
                    platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
                }
            }
        })
    }
}

fn redraw<F: FnMut(&mut bool, &Display, &mut Renderer, &mut Ui) + 'static>(
    imgui: &mut Context,
    run_ui: &mut F,
    display: &Display,
    renderer: &mut Renderer,
    control_flow: &mut ControlFlow,
    platform: &mut WinitPlatform,
) -> anyhow::Result<()> {
    let ui = imgui.frame();

    let mut run = true;
    run_ui(&mut run, display, renderer, ui);
    if !run {
        *control_flow = ControlFlow::Exit;
    }

    let gl_window = display.gl_window();
    let mut target = display.draw();
    target.clear_color_srgb(0.5, 0.5, 0.5, 1.0);
    platform.prepare_render(ui, gl_window.window());
    let draw_data = imgui.render();

    renderer
        .render(&mut target, draw_data)
        .context("Rendering failed")?;

    target.finish().context("Failed to swap buffers")?;

    Ok(())
}
