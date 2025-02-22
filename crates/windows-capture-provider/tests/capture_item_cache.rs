//! Tests for the CaptureItemCache
//!

use windows_capture_provider::{CaptureItemCache, DirectX, Monitor};

#[test]
fn create_capture_item() {
    let direct_x = DirectX::new().unwrap();

    let monitor = Monitor::get_hovered_monitor(&direct_x)
        .unwrap()
        .expect("Monitor should be some");

    let capture_item = CaptureItemCache::create_capture_item(monitor.handle.0).unwrap();
    let _size = capture_item.Size().unwrap();
}

#[test]
fn cache_active() {
    let direct_x = DirectX::new().unwrap();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    assert!(!monitors.is_empty());

    let mut cache = CaptureItemCache::new();
    cache.cache_active(&direct_x).unwrap();

    for monitor in monitors {
        assert!(cache.contains(monitor.handle.0));
    }
}

#[test]
fn prune() {
    let direct_x = DirectX::new().unwrap();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    assert!(!monitors.is_empty());

    let mut cache = CaptureItemCache::new();
    cache.cache_active(&direct_x).unwrap();
    cache.prune(&direct_x).unwrap();

    for monitor in monitors {
        assert!(cache.contains(monitor.handle.0));
    }
}

#[test]
fn get_capture_item_uncached() {
    let direct_x = DirectX::new().unwrap();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    assert!(!monitors.is_empty());

    let mut cache = CaptureItemCache::new();

    for monitor in monitors {
        let _capture_item = cache.get_capture_item(monitor.handle.0).unwrap();
    }
}

#[test]
fn get_capture_item_cached() {
    let direct_x = DirectX::new().unwrap();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    assert!(!monitors.is_empty());

    let mut cache = CaptureItemCache::new();
    cache.cache_active(&direct_x).unwrap();

    for monitor in monitors {
        let _capture_item = cache.get_capture_item(monitor.handle.0).unwrap();
    }
}
