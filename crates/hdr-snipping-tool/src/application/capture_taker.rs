use core::{
    fmt::{Debug, Display},
    time::Duration,
};
use std::{
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use tracing::{debug, error, info_span};
use vulkan::{HdrImage, HdrScanner, HistogramGenerator, Vulkan, BIN_COUNT};
use windows::Win32::Foundation::CloseHandle;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, MonitorError, WindowsCapture};
use winit::event_loop::EventLoopProxy;

use crate::utilities::failure::{report, Failure};

use super::WindowMessage;

pub enum CaptureProgress {
    FoundMonitor(Monitor),
    CaptureTaken(WindowsCapture),
    Imported(HdrImage),
    FoundWhitepoint(f32),
    Failed,
}
impl Debug for CaptureProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FoundMonitor(_) => write!(f, "FoundMonitor"),
            Self::CaptureTaken(_) => write!(f, "CaptureTaken"),
            Self::Imported(_) => write!(f, "Imported"),
            Self::FoundWhitepoint(_) => write!(f, "FoundWhitepoint"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}
impl Display for CaptureProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

enum Message {
    TakeCapture(EventLoopProxy<WindowMessage>),
    CloseHandle(WindowsCapture),
    RefreshCache,
    Shutdown,
}

pub struct CaptureTaker {
    // Option allows for joining the thread which requires ownership.
    thread: Option<JoinHandle<()>>,
    sender: Sender<Message>,
}

impl CaptureTaker {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let (sender, receiver) = channel();

        // Start the thread to handle taking the capture
        let thread = thread::Builder::new()
            .name("Capture Taker".into())
            .spawn(move || {
                let _span = info_span!("[Capture Taker]").entered();
                let mut inner = InnerCaptureTaker::new(vulkan);

                loop {
                    // unwrap should never happen, CaptureTaker owns the sender and calls shutdown on drop.
                    let message = receiver.recv().unwrap();

                    match message {
                        Message::Shutdown => break,
                        Message::RefreshCache => inner.refresh_cache(),
                        Message::TakeCapture(proxy) => inner.take_capture(proxy),
                        Message::CloseHandle(capture) => inner.close_handle(capture),
                    }
                }

                inner.shutdown();
                drop(inner);
            })
            .report_and_panic("Could not start the capture taker thread");

        // Start the thread to request cache refresh
        {
            let sender = sender.clone();
            thread::Builder::new()
                .name("Refresh Cache".into())
                .spawn(move || {
                    let _span = info_span!("[Refresh Cache]").entered();

                    loop {
                        if sender.send(Message::RefreshCache).is_err() {
                            break;
                        }

                        thread::sleep(Duration::from_secs(60 * 10));
                    }
                })
                .report_and_panic("Could not start the cache thread");
        };

        Self {
            thread: Some(thread),
            sender,
        }
    }

    pub fn take_capture(&self, proxy: EventLoopProxy<WindowMessage>) -> Result<(), ()> {
        if let Err(e) = self.sender.send(Message::TakeCapture(proxy)) {
            error!("Failed to send message to capture taker: {e}");
            return Err(());
        }

        Ok(())
    }

    pub fn close_handle(&self, capture: WindowsCapture) -> Result<(), ()> {
        if let Err(e) = self.sender.send(Message::CloseHandle(capture)) {
            error!("Failed to send message to capture taker: {e}");
            return Err(());
        }

        Ok(())
    }
}

impl Drop for CaptureTaker {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Shutdown);
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                error!("Joining Capture Taker thread returned an error.")
            };
        }
    }
}

struct InnerCaptureTaker {
    direct_x: DirectX,
    cache: CaptureItemCache,

    vulkan: Arc<Vulkan>,

    hdr_scanner: HdrScanner,
    histogram_generator: HistogramGenerator,
}

impl InnerCaptureTaker {
    pub fn new(vulkan: Arc<Vulkan>) -> Self {
        let direct_x = DirectX::new().report_and_panic("Could not create DirectX devices");
        let cache = CaptureItemCache::new();

        let hdr_scanner = unsafe { HdrScanner::new(vulkan.clone()) }
            .report_and_panic("Could not create the HDR Scanner");

        let histogram_generator = unsafe { HistogramGenerator::new(vulkan.clone()) }
            .report_and_panic("Could not create the Histogram Generator");

        Self {
            direct_x,
            cache,
            vulkan,
            hdr_scanner,
            histogram_generator,
        }
    }

    pub fn shutdown(&mut self) {}

    pub fn refresh_cache(&mut self) {
        if let Err(e) = self.cache.prune(&self.direct_x) {
            match e {
                MonitorError::WinError(win_error) => {
                    error!("Could not prune the cache: {win_error}")
                }
                MonitorError::MonitorsMismatch => {
                    debug!("Could not prune the cache: {e}");
                }
            };
        };

        if let Err(e) = self.cache.cache_active(&self.direct_x) {
            match e {
                MonitorError::WinError(win_error) => {
                    error!("Could not cache the active monitors: {win_error}")
                }
                MonitorError::MonitorsMismatch => {
                    debug!("Could not cache the active monitors: {e}");
                }
            };
        };
    }

    pub fn take_capture(&mut self, proxy: EventLoopProxy<WindowMessage>) {
        // Get the monitor
        let monitor = {
            let maybe_monitor = match Monitor::get_hovered_monitor(&self.direct_x) {
                Ok(maybe_monitor) => maybe_monitor,
                Err(e) => match e {
                    MonitorError::WinError(win_error) => {
                        report(win_error, "Could not take the screenshot.\nAn error was encountered while finding the hovered monitor");
                        let _ = proxy
                            .send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                        return;
                    }

                    MonitorError::MonitorsMismatch => {
                        report(e, "Could not take the screenshot.\nThe monitor config changed during loading");
                        let _ = proxy
                            .send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                        return;
                    }
                },
            };

            let monitor = match maybe_monitor {
                Some(monitor) => monitor,
                None => {
                    report("Monitor::get_hovered_monitor was None", "Could not take the screenshot.\nCould not find the monitor that the cursor is on");
                    let _ =
                        proxy.send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                    return;
                }
            };

            debug!("Found monitor: {monitor}");

            proxy
                .send_event(WindowMessage::CaptureProgress(
                    CaptureProgress::FoundMonitor(monitor),
                ))
                .report_and_panic("Eventloop exited");

            monitor
        };

        // Take the capture
        let (windows_capture, windows_capture_resources) = {
            let start = Instant::now();

            let capture_item = match self.cache.get_capture_item(monitor) {
                Ok(capture_item) => capture_item,
                Err(e) => {
                    report(
                        e,
                        "Could not take the screenshot.\nEncountered an error while creating the required resources",
                    );
                    let _ =
                        proxy.send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                    return;
                }
            };

            debug!("Got capture item in {}ms", start.elapsed().as_millis());
            let start = Instant::now();

            let (capture, resources) = match WindowsCapture::take_capture(
                &self.direct_x,
                monitor,
                &capture_item,
            ) {
                Ok(capture) => capture,
                Err(e) => {
                    report(e, "Could not take the screenshot.\nEncountered an error while taking the screenshot");
                    let _ =
                        proxy.send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                    return;
                }
            };

            debug!("Took capture in {}ms", start.elapsed().as_millis());

            proxy
                .send_event(WindowMessage::CaptureProgress(
                    CaptureProgress::CaptureTaken(capture),
                ))
                .report_and_panic("Eventloop exited");

            (capture, resources)
        };

        // Import the capture
        let hdr_capture = unsafe {
            let capture = match HdrImage::import_windows_capture(
                &self.vulkan,
                windows_capture.size,
                windows_capture.handle.0 .0 as isize,
            ) {
                Ok(capture) => capture,
                Err(e) => {
                    let _ = windows_capture_resources.destroy(&self.direct_x);

                    report(
                        e,
                        "Could not take the screenshot.\nEncountered an error while importing the screenshot from DirectX to the application",
                    );
                    let _ =
                        proxy.send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                    return;
                }
            };

            proxy
                .send_event(WindowMessage::CaptureProgress(CaptureProgress::Imported(
                    capture,
                )))
                .report_and_panic("Eventloop exited");

            capture
        };

        // Find the whitepoint
        {
            let maximum = match unsafe { self.hdr_scanner.scan(hdr_capture) } {
                Ok(maximum) => maximum,
                Err(e) => {
                    report(e, "Encountered an error while analysing the screenshot");
                    let _ =
                        proxy.send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                    return;
                }
            };

            debug!("Found maximum: {}", maximum);

            let is_hdr = maximum > monitor.sdr_white;

            if !is_hdr {
                debug!("Selected SDR whitepoint: {}", monitor.sdr_white);

                proxy
                    .send_event(WindowMessage::CaptureProgress(
                        CaptureProgress::FoundWhitepoint(monitor.sdr_white),
                    ))
                    .report_and_panic("Eventloop exited");
            }
            /*  else if maximum <= monitor.max_brightness {
                debug!("Selected HDR Whitepoint {}", monitor.max_brightness);

                proxy
                    .send_event(WindowMessage::CaptureProgress(
                        CaptureProgress::FoundWhitepoint(monitor.max_brightness),
                    ))
                    .report_and_panic("Eventloop exited");
            }  */
            else {
                let histogram = match unsafe {
                    self.histogram_generator.generate(hdr_capture, maximum)
                } {
                    Ok(histogram) => histogram,
                    Err(e) => {
                        report(e, "Encountered an error while analysing the screenshot");
                        let _ = proxy
                            .send_event(WindowMessage::CaptureProgress(CaptureProgress::Failed));
                        return;
                    }
                };

                // if should_debug() {
                //     let mut message = String::from("Histogram:");
                //     for (index, count) in histogram.iter().enumerate() {
                //         message += &format!("\n[{:3}] {count}", index);
                //     }
                //     debug!("{message}");
                // }

                let histogram_whitepoint = {
                    let threshold: f64 = 0.9999;

                    let total = windows_capture.size[0] * windows_capture.size[1] * 3;

                    let mut running_total = 0;
                    let mut selected_bin_index = BIN_COUNT as usize;

                    for (index, count) in histogram.iter().enumerate() {
                        running_total += count;

                        if running_total as f64 / total as f64 >= threshold {
                            debug!("Selected bin: {index}");
                            selected_bin_index = index;
                            break;
                        }
                    }

                    (maximum / BIN_COUNT as f32) * (selected_bin_index + 1) as f32
                };

                debug!("Selected HDR Whitepoint {}", histogram_whitepoint);

                proxy
                    .send_event(WindowMessage::CaptureProgress(
                        CaptureProgress::FoundWhitepoint(histogram_whitepoint),
                    ))
                    .report_and_panic("Eventloop exited");
            }
        }

        // Clean up
        {
            if let Err(e) = windows_capture_resources.destroy(&self.direct_x) {
                error!("Failed to destroy Windows Capture Resources: {e}");
            }
        }
    }

    pub fn close_handle(&self, capture: WindowsCapture) {
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
