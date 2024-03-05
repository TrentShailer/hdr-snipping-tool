use std::num::NonZeroU32;

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use raw_window_handle::HasRawWindowHandle;
use snafu::{OptionExt, ResultExt, Whatever};
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    platform::windows::IconExtWindows,
    window::{Fullscreen, Window, WindowBuilder},
};

use super::imgui_winit_support::{HiDpiMode, WinitPlatform};
use super::GuiBackendEvent;

pub fn create_window(
    event_loop: &mut EventLoop<GuiBackendEvent>,
    context_api: Option<ContextApi>,
) -> Result<(Window, Surface<WindowSurface>, PossiblyCurrentContext), Whatever> {
    let window_icon = winit::window::Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))
        .whatever_context("failed to load icon")?;

    let window_builder = WindowBuilder::new()
        .with_title("HDR Snipping Tool")
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_visible(false)
        .with_window_icon(Some(window_icon));

    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(window_builder))
        .build(&event_loop, ConfigTemplateBuilder::new(), |mut configs| {
            configs.next().unwrap()
        })
        .whatever_context("Failed to create OpenGL window")?;

    let window = window.whatever_context("Failed to create window")?;

    let mut context_attribs = ContextAttributesBuilder::new();
    if let Some(context_api) = context_api {
        context_attribs = context_attribs.with_context_api(context_api);
    }
    let context_attribs = context_attribs.build(Some(window.raw_window_handle()));
    let context = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .expect("Failed to create OpenGL context")
    };

    let window_monitor = window
        .current_monitor()
        .whatever_context("Failed to get window's monitor")?;

    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window.raw_window_handle(),
            NonZeroU32::new(window_monitor.size().width).unwrap(),
            NonZeroU32::new(window_monitor.size().height).unwrap(),
        );

    let surface = unsafe {
        cfg.display()
            .create_window_surface(&cfg, &surface_attribs)
            .expect("Failed to create OpenGL surface")
    };

    let context = context
        .make_current(&surface)
        .expect("Failed to make OpenGL context current");

    surface
        .set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
        .expect("Failed to set swap interval");

    Ok((window, surface, context))
}
pub fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}

pub fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(imgui_context.io_mut(), window, HiDpiMode::Rounded);

    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}

pub fn init_tray_icon() -> Result<TrayIcon, Whatever> {
    let icon = tray_icon::Icon::from_resource(1, Some((24, 24)))
        .whatever_context("failed to build icon")?;

    let quit_item = MenuItem::with_id(0, "Quit HDR Snipping Tool", true, None);
    let reload_item = MenuItem::with_id(1, "Reload HDR Snipping Tool", true, None);

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
