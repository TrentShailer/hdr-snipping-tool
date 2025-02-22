//! Test for DirectX functions
//!

use windows::Win32::Graphics::Dxgi::{Common::DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ENUM_MODES};
use windows_capture_provider::DirectX;

#[test]
fn create_direct_x() {
    let direct_x = DirectX::new().unwrap();
    drop(direct_x);
}

#[test]
fn dxgi_outputs() {
    let direct_x = DirectX::new().unwrap();
    let outputs = direct_x.dxgi_outputs().unwrap();

    assert!(!outputs.is_empty(), "At least one dxgi output must exist");

    for output in outputs {
        let format = DXGI_FORMAT_B8G8R8A8_UNORM;
        let flags = DXGI_ENUM_MODES::default();
        let mut modes = 0;

        unsafe {
            output
                .GetDisplayModeList(format, flags, &mut modes, None)
                .unwrap()
        };
    }
}

#[test]
fn dxgi_output_descriptors() {
    let direct_x = DirectX::new().unwrap();
    let descriptors = direct_x.dxgi_output_descriptors().unwrap();

    assert!(
        !descriptors.is_empty(),
        "At least one dxgi output must exist"
    );

    for descriptor in descriptors {
        assert!(!descriptor.Monitor.is_invalid())
    }
}
