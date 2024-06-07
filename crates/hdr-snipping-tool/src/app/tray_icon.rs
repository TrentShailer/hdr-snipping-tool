use thiserror::Error;
use tray_icon::{
    menu::{Menu, MenuItem},
    BadIcon, Icon, TrayIcon, TrayIconBuilder,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid Icon:\n{0}")]
    Icon(#[from] BadIcon),

    #[error("Failed to build tray icon:\n{0}")]
    BuildTrayIcon(#[from] tray_icon::Error),

    #[error("Failed to create menu:\n{0}")]
    Menu(#[from] tray_icon::menu::Error),
}

pub fn init_tray_icon() -> Result<TrayIcon, Error> {
    let icon = Icon::from_resource(1, Some((24, 24)))?;
    let quit_item = MenuItem::with_id(0, "Quit HDR Snipping Tool", true, None);
    let tray_menu = Menu::with_items(&[&quit_item])?;
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("HDR Snipping Tool")
        .with_icon(icon)
        .build()?;
    Ok(tray_icon)
}
