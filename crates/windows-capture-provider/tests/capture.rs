//! Tests for capture
//!

use windows::Win32::Foundation::CloseHandle;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

#[test]
fn take_capture() {
    let direct_x = DirectX::new().unwrap();

    let monitor = Monitor::get_hovered_monitor(&direct_x)
        .unwrap()
        .expect("Monitor should be some");

    let mut cache = CaptureItemCache::new();
    let capture_item = cache.get_capture_item(monitor.handle.0).unwrap();

    let (capture, resources) = WindowsCapture::take_capture(&direct_x, &capture_item).unwrap();
    assert!(!capture.handle.0.is_invalid());
    unsafe { CloseHandle(capture.handle.0).unwrap() };
    unsafe { resources.destroy(&direct_x).unwrap() }
}
