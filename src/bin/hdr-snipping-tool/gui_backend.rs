use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::platform::run_return::EventLoopExtRunReturn;
use glium::glutin::platform::windows::IconExtWindows;
use glium::glutin::window::{Fullscreen, WindowBuilder};
use glium::{Display, Surface};
use hdr_snipping_tool::PhysicalSize;
use imgui::{Context, FontConfig, FontSource, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use snafu::{Report, ResultExt, Whatever};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

#[derive(Debug, PartialEq)]
pub enum GuiBackendEvent {
    ShowWindow,
    HideWindow,
}

pub struct GuiBackend<'a> {
    pub event_loop: &'a mut EventLoop<GuiBackendEvent>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
    pub tray_icon: TrayIcon,
}

/// Initialize the app window and tray icon
pub fn init(event_loop: &mut EventLoop<GuiBackendEvent>) -> Result<GuiBackend, Whatever> {
    let context = glutin::ContextBuilder::new().with_vsync(true);

    let window_icon =
        glium::glutin::window::Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))
            .whatever_context("failed to load icon")?;

    let window_builder = WindowBuilder::new()
        .with_title("HDR Snipping Tool")
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_visible(false)
        .with_window_icon(Some(window_icon));

    let display = Display::new(window_builder, context, event_loop)
        .whatever_context("Failed to initialize display.")?;

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();

        let dpi_mode = HiDpiMode::Default;

        platform.attach_window(imgui.io_mut(), window, dpi_mode);
    }

    let font_size = 14.0;

    imgui.fonts().add_font(&[FontSource::TtfData {
        data: include_bytes!("./fonts/Inter-Regular.ttf"),
        size_pixels: font_size,
        config: Some(FontConfig {
            rasterizer_multiply: 1.5,
            oversample_h: 4,
            oversample_v: 4,
            ..FontConfig::default()
        }),
    }]);

    let renderer =
        Renderer::init(&mut imgui, &display).whatever_context("Failed to initialize renderer.")?;

    let tray_icon = init_tray_icon().whatever_context("Failed to initialize tray icon")?;
    tray_icon
        .set_visible(true)
        .whatever_context("Failed to show tray icon")?;

    Ok(GuiBackend {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
        tray_icon,
    })
}

fn init_tray_icon() -> Result<TrayIcon, Whatever> {
    let icon = tray_icon::Icon::from_resource(1, Some((24, 24)))
        .whatever_context("failed to build icon")?;

    let quit_item = MenuItem::with_id(0, "Quit HDR Snipping Tool", true, None);
    let reload_item = MenuItem::with_id(1, "Reload GUI", true, None);

    let tray_menu =
        Menu::with_items(&[&reload_item, &quit_item]).whatever_context("Failed to build menu")?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("HDR Snipping Tool")
        .with_icon(icon)
        .build()
        .whatever_context("Failed to build tray icon")?;

    Ok(tray_icon)
}

impl<'a> GuiBackend<'a> {
    pub fn main_loop<F: FnMut(&mut bool, &Display, &mut Renderer, &mut Ui) + 'static>(
        self,
        mut run_ui: F,
    ) -> i32 {
        let GuiBackend {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;

        let exit_code = event_loop.run_return(|event, _, control_flow| {
            if !display.gl_window().window().is_visible().unwrap() {
                *control_flow = glutin::event_loop::ControlFlow::Wait;
            } else {
                *control_flow = glutin::event_loop::ControlFlow::Poll;
            }

            if let Ok(tray_event) = MenuEvent::receiver().try_recv() {
                match tray_event.id.0.as_str() {
                    "0" => *control_flow = ControlFlow::ExitWithCode(0),
                    "1" => *control_flow = ControlFlow::ExitWithCode(1),
                    _ => {}
                };
            }

            match &event {
                Event::NewEvents(_) => {}
                Event::MainEventsCleared => {
                    let gl_window = display.gl_window();
                    if let Err(e) = platform
                        .prepare_frame(imgui.io_mut(), gl_window.window())
                        .whatever_context::<_, Whatever>("Failed to prepare frame")
                    {
                        log::error!("{}", Report::from_error(e).to_string());
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
                    .whatever_context::<_, Whatever>("Failed to redraw.")
                    {
                        log::error!("{}", Report::from_error(e).to_string());
                        *control_flow = ControlFlow::Exit;
                    };
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                Event::UserEvent(v) => match v {
                    GuiBackendEvent::ShowWindow => {
                        display.gl_window().window().set_visible(true);
                        display.gl_window().window().focus_window();
                    }
                    GuiBackendEvent::HideWindow => {
                        display.gl_window().window().set_visible(false);
                        imgui
                            .io_mut()
                            .add_mouse_button_event(imgui::MouseButton::Left, false);
                    }
                },
                Event::WindowEvent {
                    window_id,
                    event: _,
                } if window_id == &display.gl_window().window().id() => {
                    // ignore events when the window is invisible
                    if !display.gl_window().window().is_visible().unwrap() {
                        return;
                    }

                    let gl_window = display.gl_window();
                    platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
                }
                event => {
                    let gl_window = display.gl_window();
                    platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
                }
            }
        });

        exit_code
    }
}

fn redraw<F: FnMut(&mut bool, &Display, &mut Renderer, &mut Ui) + 'static>(
    imgui: &mut Context,
    run_ui: &mut F,
    display: &Display,
    renderer: &mut Renderer,
    control_flow: &mut ControlFlow,
    platform: &mut WinitPlatform,
) -> Result<(), Whatever> {
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
        .whatever_context("Rendering failed")?;

    target.finish().whatever_context("Failed to swap buffers")?;

    Ok(())
}
