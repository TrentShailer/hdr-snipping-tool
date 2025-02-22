//! Tests for Montior functions

use windows_capture_provider::{DirectX, Monitor};

#[test]
fn get_active_monitors() {
    let direct_x = DirectX::new().unwrap();

    let monitors = Monitor::get_active_monitors(&direct_x).unwrap();
    assert!(!monitors.is_empty(), "At least one monitor must be active");

    for monitor in monitors {
        assert!(!monitor.handle.0.is_invalid())
    }
}

#[test]
fn get_hovered_monitor() {
    let direct_x = DirectX::new().unwrap();

    let monitor = Monitor::get_hovered_monitor(&direct_x)
        .unwrap()
        .expect("Monitor should be some");

    assert!(!monitor.handle.0.is_invalid())
}
