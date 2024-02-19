#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod gui_backend;
mod hotkey;
mod logger;
mod settings;

use std::sync::{mpsc::channel, Arc};

use app::App;
use glium::glutin::event_loop::{EventLoop, EventLoopBuilder};
use gui_backend::GuiBackendEvent;
use hdr_snipping_tool::CaptureProvider;
use hotkey::init_hotkey;
use settings::Settings;
use snafu::{Report, ResultExt, Whatever};

#[cfg(windows)]
fn main() {
    logger::init_fern().unwrap();

    if let Err(e) = windows::run() {
        log::error!("{}", Report::from_error(e).to_string());
    }
}

#[cfg(windows)]
mod windows {
    use hdr_snipping_tool::WindowsCapture;
    use snafu::{whatever, ResultExt, Whatever};
    use windows::{
        core::{HRESULT, PCWSTR},
        Graphics::Capture::GraphicsCaptureSession,
        Win32::System::Threading::{CreateMutexW, OpenMutexW, MUTEX_ALL_ACCESS},
    };

    use crate::start_app;

    const MUTEX_NAME: &str = "HDR-Snipping-Tool-Process-Mutex\0";

    /// Checks if another instance of the app is running by using a Windows system mutex
    fn is_first_instance() -> Result<bool, Whatever> {
        unsafe {
            let result = OpenMutexW(
                MUTEX_ALL_ACCESS,
                true,
                PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr()),
            );

            // Some mutex with that name already exists, we aren't the first instance
            if result.is_ok() {
                return Ok(false);
            }

            let err = result.err().unwrap();
            // 0x80070002 is the error code for if no mutex with that names exists, any other error should be reported
            if err.code() != HRESULT(0x80070002u32 as i32) {
                whatever!("Failed to open single instance mutex: {err}");
            }

            // Since no mutex exists, we should create it
            CreateMutexW(
                None,
                true,
                PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr()),
            )
            .whatever_context("Failed to create mutex")?;
        }

        Ok(true)
    }

    pub fn run() -> Result<(), Whatever> {
        if !is_first_instance().whatever_context("Failed to ensure single instance")? {
            log::warn!("Another instance is already running.");
            return Ok(());
        }

        if !GraphicsCaptureSession::IsSupported().unwrap() {
            whatever!("Graphics capture is not supported.");
        }

        let capture_provider = WindowsCapture {};

        start_app(capture_provider).whatever_context("Failed to start app")?;

        Ok(())
    }
}

#[cfg(not(windows))]
fn main() {
    logger::init_fern().unwrap();
    log::error!("Your platform is not currently supported.");
}

pub fn start_app<C>(capture_provider: C) -> Result<(), Whatever>
where
    C: CaptureProvider + Send + Sync + 'static,
{
    let capture_provider = Arc::new(capture_provider);
    let mut event_loop: EventLoop<GuiBackendEvent> = EventLoopBuilder::with_user_event().build();

    loop {
        let settings = Settings::load().whatever_context("Failed to load settings")?;
        let window = gui_backend::init(&mut event_loop)
            .whatever_context("Failed to initialize app window")?;

        let (capture_sender, capture_receiver) = channel();
        let event_proxy = window.event_loop.create_proxy();
        let _hotkey_hook = init_hotkey(
            settings.screenshot_key,
            Arc::clone(&capture_provider),
            capture_sender,
            event_proxy,
        )
        .whatever_context("Failed to initialize hotkey")?;

        let event_proxy = window.event_loop.create_proxy();
        let mut app = App::new(capture_receiver, event_proxy);

        let exit_code = window.main_loop(move |run, display, renderer, ui| {
            if let Err(e) = app
                .update(ui, display, renderer)
                .whatever_context::<_, Whatever>("Failure in update loop")
            {
                log::error!("{}", Report::from_error(e).to_string());
                *run = false;
            };
        });

        if exit_code == 0 {
            break;
        }
    }

    Ok(())
}
