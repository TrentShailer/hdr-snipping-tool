use std::sync::{mpsc::Sender, Arc};

use hdr_capture::{CaptureProvider, DisplayInfo, HdrCapture};
use livesplit_hotkey::{Hook, Hotkey, KeyCode};
use snafu::{Report, ResultExt, Whatever};
use winit::event_loop::EventLoopProxy;

use super::gui_backend::GuiBackendEvent;

pub fn init_hotkey<C>(
    hotkey: KeyCode,
    capture_provider: Arc<C>,
    sender: Arc<Sender<(HdrCapture, DisplayInfo)>>,
    proxy: EventLoopProxy<GuiBackendEvent>,
) -> Result<Hook, Whatever>
where
    C: CaptureProvider + Send + Sync + 'static,
{
    let hook = Hook::new().whatever_context("Failed to create hotkey hook")?;

    hook.register(Hotkey::from(hotkey), move || {
        if let Err(e) = handle_capture(capture_provider.as_ref(), sender.as_ref(), &proxy)
            .whatever_context::<_, Whatever>("Failed to handle capture")
        {
            log::error!("{}", Report::from_error(e).to_string());
        }
    })
    .whatever_context("Failed to register hotkey")?;

    Ok(hook)
}

pub fn handle_capture<C>(
    capture_provider: &C,
    sender: &Sender<(HdrCapture, DisplayInfo)>,
    proxy: &EventLoopProxy<GuiBackendEvent>,
) -> Result<(), Whatever>
where
    C: CaptureProvider + 'static,
{
    let capture = capture_provider
        .take_capture()
        .whatever_context("Failed to take capture")?;
    sender
        .send(capture)
        .whatever_context("Failed to send capture")?;
    proxy
        .send_event(GuiBackendEvent::ShowWindow)
        .whatever_context("failed to send window event")?;
    Ok(())
}
