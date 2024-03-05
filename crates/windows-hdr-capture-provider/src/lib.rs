mod d3d_device;
mod get_display;
mod prepare_capture;
mod process_frame;

use hdr_capture::{CaptureProvider, DisplayInfo, HdrCapture};
use snafu::{ResultExt, Snafu};

use self::{
    d3d_device::create_d3d_devices, get_display::get_hovered_display,
    prepare_capture::prepare_capture, process_frame::process_frame,
};

pub struct WindowsCapture;

impl CaptureProvider for WindowsCapture {
    type Error = Error;

    fn take_capture(&self) -> Result<(HdrCapture, DisplayInfo), Self::Error> {
        let display = get_hovered_display().context(GetDisplaySnafu)?;

        let (dxgi_device, d3d_device, d3d_context) =
            create_d3d_devices().context(D3dDeviceSnafu)?;

        let (framepool, capture_session, capture_receiver) =
            prepare_capture(&display, &dxgi_device).context(FramepoolSnafu)?;

        let frame = capture_receiver.recv().unwrap();

        let capture = process_frame(frame, &d3d_device, &d3d_context).context(ProcessFrameSnafu)?;

        capture_session.Close().context(WindowsApiSnafu {
            call: "capture_session.Close()",
        })?;
        framepool.Close().context(WindowsApiSnafu {
            call: "framepool.Close()",
        })?;

        Ok((capture, display.info))
    }
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed get the display to capture"))]
    GetDisplay { source: get_display::Error },

    #[snafu(display("Failed to create d3d devices"))]
    D3dDevice { source: d3d_device::Error },

    #[snafu(display("Failed to prepare framepool"))]
    Framepool { source: prepare_capture::Error },

    #[snafu(display("Failed to process frame"))]
    ProcessFrame { source: process_frame::Error },

    #[snafu(display("Windows API call '{call}' returned an error."))]
    WindowsApi {
        source: windows::core::Error,
        call: &'static str,
    },
}
