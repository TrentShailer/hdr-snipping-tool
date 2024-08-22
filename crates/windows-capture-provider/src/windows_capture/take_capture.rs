use tracing::{info_span, instrument};
use windows::Graphics::Capture::GraphicsCaptureItem;

use crate::{DirectXDevices, Display};

use super::{
    retrieve_handle::retrieve_handle, start_capture_session::start_capture_session, Error,
    WindowsCapture,
};

impl WindowsCapture {
    #[instrument("WindowsCapture::take_capture", skip_all, err)]
    pub fn take_capture(
        devices: &DirectXDevices,
        display: (Display, GraphicsCaptureItem),
    ) -> Result<Self, Error> {
        let (display, capture_item) = display;

        // get the framepool, capture session, and captuire receiver
        let (framepool, capture_session, capture_receiver) =
            start_capture_session(devices, &capture_item)?;

        // get the d3d_capture
        let recv_span = info_span!("recv").entered();
        let d3d11_capture = capture_receiver.recv().unwrap();
        recv_span.exit();

        capture_session.Close().map_err(Error::Cleanup)?;
        framepool.Close().map_err(Error::Cleanup)?;

        // get the capture from gpu
        let capture_handle = retrieve_handle(d3d11_capture).map_err(Error::RetrieveHandle)?;

        // free resources
        unsafe { devices.d3d11_context.ClearState() };
        devices.d3d_device.Trim().map_err(Error::Cleanup)?;

        Ok(WindowsCapture {
            handle: capture_handle,
            size: display.size,
            display,
        })
    }
}
