extern crate alloc;

use alloc::sync::Arc;

use testing::setup_logger;
use tracing::info;
use vulkan::{HdrImage, HdrScanner, Vulkan};
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor, WindowsCapture};

fn main() {
    let _logger = setup_logger().unwrap();

    let vulkan = Arc::new(unsafe { Vulkan::new(true, None) }.unwrap());
    let dx = DirectX::new().unwrap();
    let mut cache = CaptureItemCache::new();
    let mut hdr_scanner = unsafe { HdrScanner::new(vulkan.clone()) }.unwrap();

    // Get capture
    let monitor = Monitor::get_hovered_monitor(&dx).unwrap().unwrap();
    let capture_item = { cache.get_capture_item(monitor).unwrap() };
    let capture = { WindowsCapture::take_capture(&dx, monitor, &capture_item).unwrap() };

    // Import capture
    let hdr_image = unsafe {
        HdrImage::import_windows_capture(
            vulkan.as_ref(),
            capture.0.size,
            capture.0.handle.0 .0 as isize,
        )
        .unwrap()
    };

    let maximum = unsafe { hdr_scanner.scan(hdr_image).unwrap() };

    info!(
        "Maximum: {} | SDR White: {:.3}",
        maximum, capture.0.monitor.sdr_white
    );

    unsafe {
        hdr_image.destroy(&vulkan);
    }

    {
        capture.1.destroy(&dx).unwrap();
    }
}
