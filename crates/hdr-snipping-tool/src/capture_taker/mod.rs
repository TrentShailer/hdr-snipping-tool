use tracing::{debug, error, info};
use utilities::DebugTime;
use vulkan::{HdrImage, HdrScanner, Vulkan};
use windows::Win32::Foundation::CloseHandle;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};
use winit::event_loop::EventLoopProxy;

use crate::{
    application::LoadingEvent,
    application_event_loop::Event,
    utilities::failure::{Failure, Ignore, report, report_and_panic},
};

pub use capture_taker_thread::CaptureTakerThread;

mod capture_taker_thread;

#[derive(Clone, Copy)]
pub enum Whitepoint {
    Sdr(f32),
    Hdr(f32),
}

impl Whitepoint {
    pub fn value(self) -> f32 {
        match self {
            Self::Sdr(value) => value,
            Self::Hdr(value) => value,
        }
    }
}

pub trait CaptureTaker {
    fn refresh_cache(&mut self);

    fn take_capture(&mut self, proxy: EventLoopProxy<Event>);

    fn cleanup_windows_capture(&self, capture: WindowsCapture);
}

pub struct BlockingCaptureTaker<'vulkan> {
    direct_x: DirectX,
    cache: CaptureItemCache,

    vulkan: &'vulkan Vulkan,

    hdr_scanner: HdrScanner<'vulkan>,
}

impl<'vulkan> BlockingCaptureTaker<'vulkan> {
    pub fn new(vulkan: &'vulkan Vulkan) -> Self {
        let direct_x = DirectX::new().report_and_panic("Could not create DirectX devices");
        let cache = CaptureItemCache::new();

        let hdr_scanner =
            HdrScanner::new(vulkan).report_and_panic("Could not create the HDR Scanner");

        Self {
            direct_x,
            cache,
            vulkan,
            hdr_scanner,
        }
    }
}

impl CaptureTaker for BlockingCaptureTaker<'_> {
    fn refresh_cache(&mut self) {
        if !self.direct_x.devices_valid() {
            report_and_panic(
                "DirectX device lost",
                "Could not refresh the cache.\nThe DirectX device was lost.",
            );
        }

        if self
            .direct_x
            .dxgi_adapter_outdated()
            .inspect_err(|e| error!("Could not check if the DirectX devices were outdated: {e}"))
            .unwrap_or(true)
        {
            self.direct_x.recreate_dxgi_adapter().report_and_panic(
                "Could not refresh the DirectX devices.\nThe DirectX device creation failed.",
            );
            self.cache.purge();
            debug!("Recreated out-of-date DXGI device, purged cache");
        }

        if let Err(e) = self.cache.prune(&self.direct_x) {
            error!("Could not prune the cache: {e}");
        };

        if let Err(e) = self.cache.cache_active(&self.direct_x) {
            error!("Could not cache the active monitors: {e}");
        };
    }

    fn take_capture(&mut self, proxy: EventLoopProxy<Event>) {
        if !self.direct_x.devices_valid() {
            report_and_panic(
                "DirectX device lost",
                "Could not refresh the cache.\nThe DirectX device was lost.",
            );
        }

        if self
            .direct_x
            .dxgi_adapter_outdated()
            .report("Could not check if the DirectX devices were outdated.")
            .unwrap_or(true)
        {
            self.direct_x.recreate_dxgi_adapter().report_and_panic(
                "Could not refresh the DirectX devices.\nThe DirectX device creation failed.",
            );
            self.cache.purge();
            debug!("Recreated out-of-date DXGI device, purged cache");
        }

        // Get the monitor
        let monitor = {
            let maybe_monitor = match Monitor::get_hovered_monitor(&self.direct_x) {
                Ok(maybe_monitor) => maybe_monitor,
                Err(e) => {
                    report(
                        e,
                        "Could not take the screenshot.\nAn error was encountered while finding the hovered monitor",
                    );
                    proxy.send_event(LoadingEvent::Error.into()).ignore();
                    return;
                }
            };

            let monitor = match maybe_monitor {
                Some(monitor) => monitor,
                None => {
                    report(
                        "Monitor::get_hovered_monitor was None",
                        "Could not take the screenshot.\nCould not find the monitor that the cursor is on",
                    );
                    proxy.send_event(LoadingEvent::Error.into()).ignore();
                    return;
                }
            };

            debug!("Hovered {monitor:?}");

            proxy
                .send_event(LoadingEvent::FoundMonitor(monitor).into())
                .report_and_panic("Eventloop exited");

            monitor
        };

        // Take the capture
        let (windows_capture, windows_capture_resources) = {
            let capture_item = {
                let _timer = DebugTime::start("Getting capture item");

                match self.cache.get_capture_item(monitor.handle.0) {
                    Ok(capture_item) => capture_item,
                    Err(e) => {
                        report(
                            e,
                            "Could not take the screenshot.\nEncountered an error while creating the required resources",
                        );
                        proxy.send_event(LoadingEvent::Error.into()).ignore();
                        return;
                    }
                }
            };

            let (capture, resources) = {
                let _timer = DebugTime::start("Taking capture");

                match WindowsCapture::take_capture(&self.direct_x, &capture_item) {
                    Ok(capture) => capture,
                    Err(e) => {
                        report(
                            e,
                            "Could not take the screenshot.\nEncountered an error while taking the screenshot",
                        );
                        proxy.send_event(LoadingEvent::Error.into()).ignore();
                        return;
                    }
                }
            };

            proxy
                .send_event(LoadingEvent::GotCapture(capture).into())
                .report_and_panic("Eventloop exited");

            (capture, resources)
        };

        // Import the capture
        let hdr_capture = unsafe {
            let capture = match HdrImage::import_windows_capture(
                self.vulkan,
                windows_capture.size,
                windows_capture.handle.0.0 as isize,
            ) {
                Ok(capture) => capture,
                Err(e) => {
                    windows_capture_resources.destroy(&self.direct_x).ignore();
                    report(
                        e,
                        "Could not take the screenshot.\nEncountered an error while importing the screenshot from DirectX to the application",
                    );
                    proxy.send_event(LoadingEvent::Error.into()).ignore();
                    return;
                }
            };

            proxy
                .send_event(LoadingEvent::ImportedCapture(capture).into())
                .report_and_panic("Eventloop exited");

            capture
        };

        // Find the whitepoint
        {
            let maximum = match unsafe { self.hdr_scanner.scan(hdr_capture) } {
                Ok(maximum) => maximum,
                Err(e) => {
                    report(e, "Encountered an error while analysing the screenshot");
                    proxy.send_event(LoadingEvent::Error.into()).ignore();
                    return;
                }
            };

            debug!("Found maximum: {}", maximum);

            let is_hdr = maximum > monitor.sdr_white;

            if !is_hdr {
                debug!("Selected SDR whitepoint: {}", monitor.sdr_white);

                proxy
                    .send_event(
                        LoadingEvent::SelectedWhitepoint(Whitepoint::Sdr(monitor.sdr_white)).into(),
                    )
                    .report_and_panic("Eventloop exited");
            } else {
                debug!("Selected HDR whitepoint: {}", monitor.max_brightness);

                proxy
                    .send_event(
                        LoadingEvent::SelectedWhitepoint(Whitepoint::Hdr(monitor.max_brightness))
                            .into(),
                    )
                    .report_and_panic("Eventloop exited");
            }
        }

        info!("Got screenshot");

        // Clean up
        {
            if let Err(e) = unsafe { windows_capture_resources.destroy(&self.direct_x) } {
                error!("Failed to destroy Windows Capture Resources: {e}");
            }
        }
    }

    fn cleanup_windows_capture(&self, capture: WindowsCapture) {
        if capture.handle.0.is_invalid() {
            return;
        }

        unsafe {
            if let Err(e) = CloseHandle(capture.handle.0) {
                error!("Failed to close handle to Windows capture: {e}");
            }
        }
    }
}
