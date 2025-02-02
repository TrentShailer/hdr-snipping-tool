use std::time::Instant;

use testing::setup_logger;
use tracing::info;
use windows_capture_provider::{CaptureItemCache, DirectX, Monitor};

fn main() {
    let _logger = setup_logger().unwrap();
    let direct_x = DirectX::new().unwrap();
    let mut cache = CaptureItemCache::new();

    let monitors = Monitor::get_monitors(&direct_x).unwrap();
    for monitor in monitors {
        println!("{monitor}");
    }

    let hovered = Monitor::get_hovered_monitor(&direct_x).unwrap().unwrap();
    println!("Hovered: {hovered}");

    let _capture_item = {
        let start = Instant::now();

        let item = cache.get_capture_item(hovered).unwrap();

        info!(
            "Getting capture item took {}ms",
            start.elapsed().as_millis()
        );

        item
    };

    let _capture_item = {
        let start = Instant::now();

        let item = cache.get_capture_item(hovered).unwrap();

        info!(
            "Getting capture item took {}ms",
            start.elapsed().as_millis()
        );

        item
    };
}
