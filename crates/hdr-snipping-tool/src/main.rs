#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod gui_backend;
mod logger;
mod settings;

use std::sync::mpsc::channel;

use app::App;

use error_trace::{ErrorTrace, ResultExt};
use gamma_compression_tonemapper::GammaCompressionTonemapper;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use gui_backend::GuiBackendEvent;
use hdr_capture::CaptureProvider;
use settings::Settings;
use winit::event_loop::{EventLoop, EventLoopBuilder};

#[cfg(windows)]
fn main() {
    logger::init_fern().unwrap();

    if let Err(e) = windows::run().track() {
        log::error!("{}", e.to_string());
    }
}

#[cfg(windows)]
mod windows {
    use error_trace::{ErrorTrace, ResultExt};
    use windows::{
        core::{HRESULT, PCWSTR},
        Graphics::Capture::GraphicsCaptureSession,
        Win32::System::Threading::{CreateMutexW, OpenMutexW, MUTEX_ALL_ACCESS},
    };
    use windows_hdr_capture_provider::WindowsCapture;

    use crate::start_app;

    const MUTEX_NAME: &str = "HDR-Snipping-Tool-Process-Mutex\0";

    /// Checks if another instance of the app is running by using a Windows system mutex
    fn is_first_instance() -> Result<bool, ErrorTrace> {
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
                return Err(err).context("Failed to open windows mutex");
            }

            // Since no mutex exists, we should create it
            CreateMutexW(
                None,
                true,
                PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr()),
            )
            .track()?;
        }

        Ok(true)
    }

    pub fn run() -> Result<(), ErrorTrace> {
        if !is_first_instance().context("Failed to ensure single instance")? {
            log::warn!("Another instance is already running.");
            return Ok(());
        }

        if !GraphicsCaptureSession::IsSupported().track()? {
            return Err("Graphics capture is not supported.").track();
        }

        let capture_provider = WindowsCapture {};

        start_app(capture_provider).track()?;

        Ok(())
    }
}

#[cfg(not(windows))]
fn main() {
    logger::init_fern().unwrap();
    log::error!("Your platform is not currently supported.");
}

pub fn start_app<C>(capture_provider: C) -> Result<(), ErrorTrace>
where
    C: CaptureProvider + Send + Sync + 'static,
{
    let mut event_loop: EventLoop<GuiBackendEvent> =
        EventLoopBuilder::with_user_event().build().track()?;

    let settings = Settings::load().context("Failed to load settings")?;
    let window = gui_backend::init(&mut event_loop).context("Failed to initialize app window")?;

    let (capture_sender, capture_receiver) = channel();

    let hotkey_manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(None, settings.screenshot_key);
    hotkey_manager.register(hotkey).track()?;

    let tonemapper = GammaCompressionTonemapper::new(settings.default_gamma);
    let event_proxy = window.event_loop.create_proxy();
    let mut app = App::new(capture_receiver, event_proxy, settings, tonemapper);

    window
        .run(
            capture_provider,
            capture_sender,
            move |ui, window, textures, gl| {
                if let Err(e) = app.update(ui, window, textures, gl).track() {
                    log::error!("{}", e.to_string());
                };
            },
        )
        .context("Failure running app")?;

    Ok(())
}
