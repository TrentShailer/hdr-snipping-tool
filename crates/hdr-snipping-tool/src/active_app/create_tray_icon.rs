use thiserror::Error;
use tracing::instrument;
use tray_icon::{
    menu::{Menu, MenuItem},
    BadIcon, Icon, TrayIcon, TrayIconBuilder,
};

use crate::{is_debug, IS_DEV, VERSION};

#[instrument(skip_all, err)]
pub fn create_tray_icon() -> Result<TrayIcon, Error> {
    let icon = Icon::from_resource(1, Some((24, 24)))?;

    let directory_item = MenuItem::with_id(0, "Open Storage Directory", true, None);
    let quit_item = MenuItem::with_id(1, "Quit HDR Snipping Tool", true, None);

    let tray_menu = Menu::with_items(&[&directory_item, &quit_item])?;

    let dev_tooltip = if IS_DEV { "-dev" } else { "" };
    let debug_tooltip = if is_debug() { "-debug" } else { "" };

    let tooltip = format!(
        "HDR Snipping Tool v{}{}{}",
        VERSION, dev_tooltip, debug_tooltip
    );

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(tooltip)
        .with_icon(icon)
        .build()?;

    Ok(tray_icon)
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid Icon:\n{0}")]
    Icon(#[from] BadIcon),

    #[error("Failed to build tray icon:\n{0}")]
    BuildTrayIcon(#[from] tray_icon::Error),

    #[error("Failed to create menu:\n{0}")]
    Menu(#[from] tray_icon::menu::Error),
}
