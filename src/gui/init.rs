use anyhow::Context;
use glium::glutin::{
    dpi::PhysicalSize,
    platform::windows::IconExtWindows,
    window::{Fullscreen, WindowBuilder},
};
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIcon, TrayIconBuilder,
};

use super::{gui, Gui};

pub fn init_gui() -> anyhow::Result<(Gui, TrayIcon)> {
    let winit_icon = load_winit_icon().context("Failed to load winit icon")?;

    let window = WindowBuilder::new()
        .with_title("Screenshot")
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .with_visible(false)
        .with_window_icon(Some(winit_icon));

    let tray_icon = init_tray_icon().context("failed to init tray icon")?;

    let gui = gui::init(window).context("Failed to create gui.")?;

    Ok((gui, tray_icon))
}

fn init_tray_icon() -> anyhow::Result<TrayIcon> {
    let icon = load_tray_icon().context("Failed to load tray icon")?;

    let item = MenuItem::with_id(0, "Quit HDR Snipping Tool", true, None);

    let tray_menu = Menu::with_items(&[&item]).context("Failed to build menu")?;

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("HDR Snipping Tool")
        .with_icon(icon)
        .build()
        .context("Failed to build tray icon")?;

    Ok(tray_icon)
}

fn load_winit_icon() -> anyhow::Result<glium::glutin::window::Icon> {
    let winit_icon = glium::glutin::window::Icon::from_resource(1, Some(PhysicalSize::new(64, 64)))
        .context("failed to load icon")?;
    Ok(winit_icon)
}

fn load_tray_icon() -> anyhow::Result<tray_icon::Icon> {
    let tray_icon =
        tray_icon::Icon::from_resource(1, Some((24, 24))).context("failed to build icon")?;
    Ok(tray_icon)
}
