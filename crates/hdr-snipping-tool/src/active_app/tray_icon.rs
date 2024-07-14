use std::process::Command;

use tray_icon::menu::MenuEvent;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use winit::event_loop::ActiveEventLoop;

use crate::{message_box::display_message, project_directory};

use super::ActiveApp;

impl ActiveApp {
    pub fn handle_tray_icon(&self, event_loop: &ActiveEventLoop) {
        let Ok(event) = MenuEvent::receiver().try_recv() else {
            return;
        };

        match event.id.0.as_str() {
            "0" => {
                if let Err(e) = Command::new("explorer").arg(project_directory()).spawn() {
                    log::error!("{e}");
                    display_message("We encountered an error while opening file explor\nMore details are in the logs.", MB_ICONERROR);
                    event_loop.exit();
                }
            }
            "1" => event_loop.exit(),
            _ => {}
        }
    }
}
