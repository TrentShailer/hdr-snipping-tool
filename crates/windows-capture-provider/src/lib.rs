pub mod d3d_devices;
pub mod display;
pub mod enumerate_displays;
pub mod fetch_capture;
pub mod get_capture;
pub mod prepare_capture;

use display::Display;
use enumerate_displays::refresh_displays;
use prepare_capture::prepare_capture;

use thiserror::Error;
use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::Graphics::{
        Direct3D11::{ID3D11Device, ID3D11DeviceContext},
        Dxgi::{IDXGIAdapter1, IDXGIDevice},
    },
};

pub struct WindowsCaptureProvider {
    _dxgi_device: IDXGIDevice,
    dxgi_adapter: IDXGIAdapter1,
    d3d_device: IDirect3DDevice,
    d3d11_device: ID3D11Device,
    d3d11_context: ID3D11DeviceContext,
    displays: Vec<Display>,
}

impl WindowsCaptureProvider {
    pub fn new() -> Result<Self, Error> {
        let (dxgi_device, dxgi_adapter, d3d_device, d3d11_device, d3d11_context) =
            d3d_devices::create_d3d_devices()?;

        let mut displays = vec![];
        refresh_displays(&dxgi_adapter, &mut displays).map_err(Error::EnumerateDisplays)?;

        Ok(Self {
            _dxgi_device: dxgi_device,
            dxgi_adapter,
            d3d_device,
            d3d11_device,
            d3d11_context,
            displays,
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create D3D Devices:\n{0}")]
    CreateDevices(#[from] d3d_devices::Error),

    #[error("Failed to enumerate displays:\n{0}")]
    EnumerateDisplays(#[source] windows_result::Error),
}
