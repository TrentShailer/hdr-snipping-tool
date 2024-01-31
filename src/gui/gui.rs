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

pub fn init(window_builder: WindowBuilder) -> Gui {
    let event_loop = EventLoopBuilder::with_user_event().build();
    let context = glutin::ContextBuilder::new().with_vsync(true);

    let display =
        Display::new(window_builder, context, &event_loop).expect("Failed to initialize display");

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

    let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    Gui {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
    }
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

        event_loop.run(move |event, _, control_flow| {
            *control_flow = glutin::event_loop::ControlFlow::Wait;

            match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => {
                    let gl_window = display.gl_window();
                    platform
                        .prepare_frame(imgui.io_mut(), gl_window.window())
                        .expect("Failed to prepare frame");
                    gl_window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let ui = imgui.frame();

                    let mut run = true;
                    run_ui(&mut run, &display, &mut renderer, ui);
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
                        .expect("Rendering failed");
                    target.finish().expect("Failed to swap buffers");
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::UserEvent(v) => v.handle(&display, &mut imgui, &mut platform, &mut renderer),
                event => {
                    let gl_window = display.gl_window();
                    platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
                }
            }
        })
    }
}
