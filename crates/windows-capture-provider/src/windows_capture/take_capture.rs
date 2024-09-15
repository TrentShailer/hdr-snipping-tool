use std::{sync::Arc, thread, time::Duration};

use tracing::{info_span, instrument};
use windows::Graphics::Capture::GraphicsCaptureItem;

use crate::{DirectXDevices, Display};

use super::{start_capture_session::start_capture_session, Error, WindowsCapture};

impl WindowsCapture {
    #[instrument("WindowsCapture::take_capture", skip_all, err)]
    pub fn take_capture(
        devices: Arc<DirectXDevices>,
        display: (Display, GraphicsCaptureItem),
    ) -> Result<Self, Error> {
        let (display, capture_item) = display;

        // get the framepool, capture session, and captuire receiver
        let (framepool, capture_session, capture_receiver) =
            start_capture_session(&devices, &capture_item)?;

        // get the d3d_capture
        let recv_span = info_span!("recv").entered();
        let handle = capture_receiver.recv().unwrap();
        recv_span.exit();

        // free resources, deferred to background thread to let main thread continue.
        thread::spawn(move || {
            // Closes must occur ASAP to prevent more frames from being fetched.
            {
                let _span = info_span!("WindowsCapture::take_capture::Close").entered();
                capture_session.Close().unwrap();
                framepool.Close().unwrap();
            }

            // Prevent Dx from interrupting performance critical Vk operations
            thread::sleep(Duration::from_millis(100));
            {
                let _span = info_span!("WindowsCapture::take_capture::Trim").entered();
                unsafe { devices.d3d11_context.ClearState() };
                (*devices.d3d_device).Trim().unwrap();
            }
        });

        Ok(WindowsCapture {
            handle,
            size: display.size,
            display,
        })
    }
}
