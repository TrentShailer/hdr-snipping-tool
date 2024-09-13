mod create_devices;

use create_devices::{d3d11_context, d3d11_device, d3d_device, dxgi_adapter};
use thiserror::Error;
use tracing::instrument;
use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::Graphics::{
        Direct3D11::{ID3D11Device, ID3D11DeviceContext},
        Dxgi::{IDXGIAdapter1, IDXGIDevice},
    },
};
use windows_core::Interface;
use windows_result::Error as WindowsError;

/// Structure containing the various directX devices used in capture aquisition.
pub struct DirectXDevices {
    /// Used to aquire display information.
    pub dxgi_adapter: IDXGIAdapter1,

    /// Used to create framepool.
    pub d3d_device: IDirect3DDevice,

    /// Used to retrieve capture from the GPU.
    pub d3d11_device: ID3D11Device,

    /// Used to retrieve capture from the GPU.
    pub d3d11_context: ID3D11DeviceContext,
}

impl DirectXDevices {
    /// Creates a new set of directX devices.
    #[instrument("DirectXDevices::new", skip_all, err)]
    pub fn new() -> Result<Self, Error> {
        let d3d11_device = d3d11_device().map_err(Error::D3D11Device)?;
        let d3d11_context = d3d11_context(&d3d11_device).map_err(Error::D3D11Context)?;

        let dxgi_device: IDXGIDevice = d3d11_device.cast().map_err(Error::DXGIAdapter)?;
        let dxgi_adapter = dxgi_adapter(&dxgi_device).map_err(Error::DXGIAdapter)?;

        let d3d_device = d3d_device(&dxgi_device).map_err(Error::D3DDevice)?;

        Ok(Self {
            dxgi_adapter,
            d3d_device,
            d3d11_device,
            d3d11_context,
        })
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to create d3d11 device:\n{0}")]
    D3D11Device(#[source] WindowsError),

    #[error("Failed to create d3d11 context:\n{0}")]
    D3D11Context(#[source] WindowsError),

    #[error("Failed to create d3d device:\n{0}")]
    D3DDevice(#[source] WindowsError),

    #[error("Failed to create dxgi adapter:\n{0}")]
    DXGIAdapter(#[source] WindowsError),
}
