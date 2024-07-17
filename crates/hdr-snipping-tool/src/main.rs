#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod active_app;
mod active_capture;
mod logger;
mod message_box;
mod only_instance;
mod settings;
mod winit_app;

use std::{fs, path::PathBuf};

use directories::ProjectDirs;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use logger::init_fern;
use message_box::display_message;
use only_instance::ensure_only_instance;
use settings::Settings;
use thiserror::Error;
use windows::{
    Graphics::Capture::GraphicsCaptureSession,
    Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_ICONWARNING},
};
use windows_capture_provider::Error;
use winit::{
    error::EventLoopError, event_loop::EventLoop, platform::run_on_demand::EventLoopExtRunOnDemand,
};
use winit_app::WinitApp;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(debug_assertions)]
pub const IS_DEV: bool = true;
#[cfg(not(debug_assertions))]
pub const IS_DEV: bool = false;

fn main() {
    if std::env::args().any(|arg| arg.eq("--debug")) {
        std::env::set_var("hdr-snipping-tool-debug", "true");
    }

    if let Err(e) = fs::create_dir_all(project_directory()) {
        display_message(
            &format!("We encountered an error while creating necessary files.\n{e}"),
            MB_ICONERROR,
        );
        return;
    };

    if let Err(e) = init_fern() {
        display_message(
            &format!("We encountered an error while setting up the logger.\n{e}"),
            MB_ICONERROR,
        );
        return;
    };

    if let Err(e) = init() {
        log::error!("{e}");

        match e {
            AppError::OnlyInstance(_) => display_message(
                "We encountered an error while checking that there were no other instances running.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::GraphicsCaptureSupport(_) => display_message(
                "We encountered an error while checking if your devices support graphics capture.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::NoCaptureSupport => display_message(
                "Your device does not support graphics capture.",
                MB_ICONERROR,
            ),
            AppError::LoadSettings(_) => display_message(
                "We encountered an error while loading your settings.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::SaveSettings(_) => display_message(
                "We encountered an error while saving your settings.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::EventLoop(_) => display_message(
                "We encountered an error in the event loop.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::Hotkey(_) => display_message(
                "We encountered an error while registering your hotkey.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
            AppError::WindowsCaptureProvider(_) => display_message(
                "We encountered an error while setting up the windows capture provider.\nMore details are in the logs.",
                MB_ICONERROR,
            ),
        }
    };
}

#[derive(Debug, Error)]
enum AppError {
    #[error("Failed to ensure only one instance:\n{0}")]
    OnlyInstance(#[from] only_instance::Error),

    #[error("Failed to check if graphics capture is supported:\n{0}")]
    GraphicsCaptureSupport(#[source] windows_result::Error),

    #[error("Graphics capture is not supported on your device.")]
    NoCaptureSupport,

    #[error("Failed to load settings:\n{0}")]
    LoadSettings(#[from] settings::LoadError),

    #[error("Failed to save settings:\n{0}")]
    SaveSettings(#[from] settings::SaveError),

    #[error("Failed to build event loop:\n{0}")]
    EventLoop(#[from] EventLoopError),

    #[error("Failed to register screenshot hotkey:\n{0}")]
    Hotkey(#[from] global_hotkey::Error),

    #[error("Failed to create windows capture provider:\n{0}")]
    WindowsCaptureProvider(#[from] Error),
}

fn init() -> Result<(), AppError> {
    // Ensure no other instances of this app are running
    if !ensure_only_instance()? {
        log::warn!("Another instance is already running.");
        display_message("Another instance is already running.", MB_ICONWARNING);
        return Ok(());
    }

    // Ensure we have graphics captuer support
    if !GraphicsCaptureSession::IsSupported().map_err(AppError::GraphicsCaptureSupport)? {
        return Err(AppError::NoCaptureSupport);
    }

    // Load or create the settings
    let settings = match Settings::load_or_create() {
        Ok(v) => v,
        Err(e) => match e {
            // If the settings file is invalid then tell the user and replace
            // it with a default one
            settings::LoadError::Deserialize(e) => {
                log::warn!("{e}");
                display_message(
                    "Invalid settings file, it will be replaced with a new one.",
                    MB_ICONWARNING,
                );
                let settings = Settings::default();
                settings.save()?;
                settings
            }
            _ => return Err(AppError::LoadSettings(e)),
        },
    };
    log::info!("{:#?}", &settings);

    // Create event loop
    let mut event_loop: EventLoop<()> = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    // Register screenshot hotkey
    let hotkey_manager = GlobalHotKeyManager::new()?;
    let hotkey = HotKey::new(None, settings.screenshot_key);
    hotkey_manager.register(hotkey)?;
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            if let Err(e) = proxy.send_event(()) {
                log::error!("Failed to send event to event loop:\n{e}");
                display_message(
                    "We encountered an error while handling your hotkey.\nMore details are in the logs.",
                    MB_ICONERROR,
                );
                std::process::exit(-1);
            }
        }
    }));

    // Create the app
    let mut app = WinitApp::new();

    // run the app
    event_loop.run_app_on_demand(&mut app)?;
    Ok(())
}

pub fn project_directory() -> PathBuf {
    let dir = match ProjectDirs::from("com", "trentshailer", "hdr-snipping-tool") {
        Some(v) => v,
        None => {
            display_message("We were unable to get the app directory.", MB_ICONERROR);
            std::process::exit(-1);
        }
    };

    dir.data_dir().to_path_buf()
}
