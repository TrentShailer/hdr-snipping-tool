//! # HDR Snipping Tool
//! Main application logic for HDR Snipping Tool.
//!

#![allow(clippy::std_instead_of_alloc)]
// hide console window on Windows in release
#![cfg_attr(feature = "hide-console", windows_subsystem = "windows")]

use application::ApplicationEvent;
use application_event_loop::{ApplicationEventLoop, Event, TrayIcon};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub use utilities::directories::{config_dir, screenshot_dir};

use config::Config;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use logger::setup_logger;
use tracing::{info, info_span, warn};
use utilities::{
    directories::create_dirs,
    failure::{Failure, Ignore, report_and_panic},
    windows_helpers::{display_message, is_first_instance},
};
use windows::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_SETFOREGROUND, MB_YESNO,
};
use winit::event_loop::EventLoop;

mod application;
mod application_event_loop;
mod capture_saver;
mod capture_taker;
mod config;
mod logger;
mod renderer_thread;
mod selection;
mod utilities;

/// The Cargo package version.
#[cfg(not(debug_assertions))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The Cargo package version or '0.0.0' if a non-release build.
#[cfg(debug_assertions)]
pub const VERSION: &str = "0.0.0";

/// If this instance should have debug enabled.
pub fn should_debug() -> bool {
    std::env::args().any(|arg| arg.eq("--debug"))
}

fn main() {
    create_dirs().report_and_panic("The config and screenshots folders could not be created");

    // Set up logger
    let _logger_guards = setup_logger(should_debug());

    // Log application start
    let _span = info_span!("[Main Thread]").entered();
    info!("HDR Snipping Tool v{}", VERSION);

    // Ensure this instance is the first instance running.
    {
        let is_fist_instance = is_first_instance()
            .report_and_panic("Could not check if HDR Snipping Tool was already running");

        if !is_fist_instance {
            warn!("Exiting: HDR Snipping Tool is already running.");
            return;
        }
    }

    // Load config
    let config = {
        let maybe_config = match Config::try_load_config() {
            Ok(maybe_config) => maybe_config,
            Err(error) => {
                warn!("Could not deserialize config file:\n{error}");
                let action = display_message(
                    "Your config file is invalid.\nMore details are in the logs.\n\nClear and reset your config file?",
                    MB_SETFOREGROUND | MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
                );

                match action {
                    // If user wants default config, maybe_config should be none for a new one to be
                    // created.
                    IDYES => {
                        info!("Resetting config file.");
                        None
                    }

                    IDNO => {
                        warn!("Exiting: Invalid config.");
                        return;
                    }

                    value => report_and_panic(
                        format!("Message box returned an unexpected response: {value:?}"),
                        "Message box returned an unexpected response",
                    ),
                }
            }
        };

        match maybe_config {
            Some(config) => config,
            None => {
                let config = Config::default();
                config.save();
                config
            }
        }
    };

    // Create event loop
    let event_loop: EventLoop<Event> = EventLoop::with_user_event()
        .build()
        .report_and_panic("Could not create the application window");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    // Register tray icon event handler
    let _tray_icon = {
        let tray_icon = TrayIcon::new();

        let proxy = event_loop.create_proxy();
        tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
            proxy.send_event(Event::TrayEvent(event)).ignore();
        }));

        tray_icon
    };

    // Register screenshot hotkey
    let _hotkey_manager = {
        let hotkey_manager =
            GlobalHotKeyManager::new().report_and_panic("Could not setup screenshot hotkey");

        let hotkey = HotKey::new(None, config.screenshot_key);
        hotkey_manager
            .register(hotkey)
            .report_and_panic("Could not register screenshot hotkey");

        hotkey_manager
    };

    // Setup hotkey event handler
    {
        let proxy = event_loop.create_proxy();

        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                info!("Hotkey pressed");
                proxy
                    .send_event(ApplicationEvent::ScreenshotKeyPressed.into())
                    .ignore();
            }
        }));
    }

    // Create the app
    let mut app = ApplicationEventLoop::new(event_loop.create_proxy());

    // Run the app
    event_loop.run_app(&mut app).ignore();
}
