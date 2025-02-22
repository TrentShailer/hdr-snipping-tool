//! Test for HdrImage

extern crate alloc;

use alloc::sync::Arc;

use vulkan::{HdrImage, HdrScanner, Vulkan};
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

#[test]
fn import_hdr_image() {
    let vulkan = Arc::new(Vulkan::new(true, None).unwrap());
    let direct_x = DirectX::new().unwrap();

    let monitor = Monitor::get_hovered_monitor(&direct_x)
        .unwrap()
        .expect("Monitor should be some");

    let mut cache = CaptureItemCache::new();
    let capture_item = cache.get_capture_item(monitor.handle.0).unwrap();

    let (capture, resources) = WindowsCapture::take_capture(&direct_x, &capture_item).unwrap();
    assert!(!capture.handle.0.is_invalid());

    let hdr_image = unsafe {
        HdrImage::import_windows_capture(&vulkan, capture.size, capture.handle.0.0 as isize)
            .unwrap()
    };

    let mut hdr_scanner = HdrScanner::new(Arc::clone(&vulkan)).unwrap();

    let _maximum = unsafe { hdr_scanner.scan(hdr_image).unwrap() };

    unsafe { hdr_image.destroy(&vulkan) };
    unsafe { resources.destroy(&direct_x).unwrap() };
}
