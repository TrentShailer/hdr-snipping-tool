use std::process::Command;

use tracing::debug;
use tray_icon::{
    TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use winit::event_loop::ActiveEventLoop;

use crate::{VERSION, config_dir, screenshot_dir, should_debug, utilities::failure::Failure};

pub const TRAY_SCREENSHOT_ID: &str = "open_screenshot_dir";
pub const TRAY_CONFIG_ID: &str = "open_config_dir";
pub const TRAY_QUIT_ID: &str = "quit";

pub struct TrayIcon {
    #[expect(unused)]
    tray_icon: tray_icon::TrayIcon,
}

impl TrayIcon {
    pub fn new() -> Self {
        let screenshot_item =
            MenuItem::with_id(TRAY_SCREENSHOT_ID, "Open Screenshot Directory", true, None);
        let config_item = MenuItem::with_id(TRAY_CONFIG_ID, "Open Config Directory", true, None);
        let quit_item = MenuItem::with_id(TRAY_QUIT_ID, "Quit HDR Snipping Tool", true, None);

        let tray_menu = Menu::with_items(&[&screenshot_item, &config_item, &quit_item])
            .report_and_panic("Could not create tray icon");
        let icon = tray_icon::Icon::from_resource(1, Some((24, 24)))
            .report_and_panic("Could not create tray icon");
        let tooltip = if should_debug() {
            format!("HDR Snipping Tool v{VERSION} (debug)")
        } else {
            format!("HDR Snipping Tool v{VERSION}")
        };

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(tooltip)
            .with_icon(icon)
            .build()
            .report_and_panic("Could not create tray icon");

        Self { tray_icon }
    }

    pub fn handle_event(event_loop: &ActiveEventLoop, event: MenuEvent) {
        debug!("Tray Event: {}", event.id.0.as_str());
        match event.id.0.as_str() {
            TRAY_SCREENSHOT_ID => {
                Command::new("explorer")
                    .arg(screenshot_dir())
                    .spawn()
                    .report_and_panic("Could not open Windows Exporer")
                    .wait()
                    .report_and_panic("Could not open Windows Exporer");
            }
            TRAY_CONFIG_ID => {
                Command::new("explorer")
                    .arg(config_dir())
                    .spawn()
                    .report_and_panic("Could not open Windows Exporer")
                    .wait()
                    .report_and_panic("Could not open Windows Exporer");
            }
            TRAY_QUIT_ID => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}
