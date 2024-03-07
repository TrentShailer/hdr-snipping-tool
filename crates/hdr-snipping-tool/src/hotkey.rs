use std::sync::{mpsc::Sender, Arc};

use error_trace::{ErrorTrace, ResultExt};
use hdr_capture::{CaptureProvider, DisplayInfo, HdrCapture};
use livesplit_hotkey::{Hook, Hotkey, KeyCode};
use winit::event_loop::EventLoopProxy;

use super::gui_backend::GuiBackendEvent;

pub fn init_hotkey<C>(
    hotkey: KeyCode,
    capture_provider: Arc<C>,
    sender: Arc<Sender<(HdrCapture, DisplayInfo)>>,
    proxy: EventLoopProxy<GuiBackendEvent>,
) -> Result<Hook, ErrorTrace>
where
    C: CaptureProvider + Send + Sync + 'static,
{
    let hook = Hook::new().track()?;

    hook.register(Hotkey::from(hotkey), move || {
        if let Err(e) = handle_capture(capture_provider.as_ref(), sender.as_ref(), &proxy)
            .context("Failed to handle capture")
        {
            log::error!("{}", e.to_string());
        }
    })
    .track()?;

    Ok(hook)
}

pub fn handle_capture<C>(
    capture_provider: &C,
    sender: &Sender<(HdrCapture, DisplayInfo)>,
    proxy: &EventLoopProxy<GuiBackendEvent>,
) -> Result<(), ErrorTrace>
where
    C: CaptureProvider + 'static,
{
    let capture = capture_provider.take_capture().track()?;
    sender.send(capture).track()?;
    proxy.send_event(GuiBackendEvent::ShowWindow).track()?;

    Ok(())
}
