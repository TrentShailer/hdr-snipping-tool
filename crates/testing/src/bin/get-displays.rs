use testing::setup_logger;
use tracing::{info, info_span};
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor};

fn main() {
    let _logger = setup_logger().unwrap();
    let direct_x = DirectX::new().unwrap();
    let mut cache = CaptureItemCache::new();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    info!("Monitors {monitors:#?}");

    let hovered = Monitor::get_hovered_monitor(&direct_x).unwrap().unwrap();
    info!("Hovered {hovered:?}");

    let _capture_item = {
        let _span = info_span!("Get item 1").entered();

        cache.get_capture_item(hovered.handle.0).unwrap()
    };

    let _capture_item = {
        let _span = info_span!("Get item 2").entered();

        cache.get_capture_item(hovered.handle.0).unwrap()
    };
}
