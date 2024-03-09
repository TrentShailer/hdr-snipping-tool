use std::num::NonZeroU32;

use error_trace::{ErrorTrace, OptionExt, ResultExt};
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use imgui::{FontConfig, FontSource};
use raw_window_handle::HasRawWindowHandle;
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
) -> Result<(Window, Surface<WindowSurface>, PossiblyCurrentContext), ErrorTrace> {
    let window_icon =
        winit::window::Icon::from_resource(1, Some(PhysicalSize::new(64, 64))).track()?;

    let window_builder = WindowBuilder::new()
        .with_title("HDR Snipping Tool")
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_visible(false)
        .with_window_icon(Some(window_icon));

    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(window_builder))
        .build(&event_loop, ConfigTemplateBuilder::new(), |mut configs| {
            configs.next().log_none().unwrap()
        })
        .track()?;

    let window = window.track()?;

    let mut context_attribs = ContextAttributesBuilder::new();
    if let Some(context_api) = context_api {
        context_attribs = context_attribs.with_context_api(context_api);
    }
    let context_attribs = context_attribs.build(Some(window.raw_window_handle()));
    let context = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .track()?
    };

    let window_monitor = window.current_monitor().track()?;

    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window.raw_window_handle(),
            NonZeroU32::new(window_monitor.size().width).track()?,
            NonZeroU32::new(window_monitor.size().height).track()?,
        );

    let surface = unsafe {
        cfg.display()
            .create_window_surface(&cfg, &surface_attribs)
            .track()?
    };

    let context = context.make_current(&surface).track()?;

    surface
        .set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).track()?))
        .track()?;

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

    let inter_font = FontSource::TtfData {
        data: include_bytes!("../fonts/Inter-Regular.ttf"),
        size_pixels: 13.5,
        config: Some(FontConfig {
            oversample_h: 4,
            oversample_v: 4,
            ..Default::default()
        }),
    };

    imgui_context.fonts().add_font(&[
        inter_font, /* imgui::FontSource::DefaultFontData { config: None } */
    ]);

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;

    (winit_platform, imgui_context)
}

pub fn init_tray_icon() -> Result<TrayIcon, ErrorTrace> {
    let icon = tray_icon::Icon::from_resource(1, Some((24, 24))).track()?;

    let quit_item = MenuItem::with_id(0, "Quit HDR Snipping Tool", true, None);

    let tray_menu = Menu::with_items(&[&quit_item]).track()?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("HDR Snipping Tool")
        .with_icon(icon)
        .build()
        .track()?;

    Ok(tray_icon)
}
