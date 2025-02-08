use tracing::error;
use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
            },
            Dxgi::{IDXGIAdapter1, IDXGIDevice, DXGI_ERROR_UNSUPPORTED},
        },
        System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
    },
};
use windows_core::{Interface, Result as WindowsResult};

use crate::{LabelledWinResult, WinError};

/// Structure containing the various directX devices used in capture aquisition.
pub struct DirectX {
    /// Used to aquire monitor information.
    pub dxgi_adapter: IDXGIAdapter1,

    /// Used to create framepool.
    pub d3d_device: IDirect3DDevice,

    /// Used to create other dx contexts.
    pub d3d11_device: ID3D11Device,

    /// Used to retrieve capture from the GPU.
    pub d3d11_context: ID3D11DeviceContext,
}

impl DirectX {
    /// Creates a new set of directX devices.
    pub fn new() -> LabelledWinResult<Self> {
        // Create the d3d11 device
        let d3d11_device = {
            let mut device = None;
            let mut result = d3d11_device_with_type(
                D3D_DRIVER_TYPE_HARDWARE,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &mut device,
            );

            if let Err(error) = &result {
                if error.code() == DXGI_ERROR_UNSUPPORTED {
                    result = d3d11_device_with_type(
                        D3D_DRIVER_TYPE_WARP,
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        &mut device,
                    );
                }
            }
            result.map_err(|e| WinError::new(e, "D3D11CreateDevice"))?;

            device.expect("d3d11_device was none")
        };

        // Get the d3d11 context.
        let d3d11_context = unsafe { d3d11_device.GetImmediateContext() }
            .map_err(|e| WinError::new(e, "ID3D11Device::GetImmediateContext"))?;

        // Cast to the dgxi device.
        let dxgi_device: IDXGIDevice = d3d11_device
            .cast()
            .map_err(|e| WinError::new(e, "ID3D11Device::cast"))?;

        // Get the adaptor.
        let dxgi_adapter = {
            let dxgi_adapter = unsafe { dxgi_device.GetAdapter() }
                .map_err(|e| WinError::new(e, "IDXGIDevice::GetAdapter"))?;

            dxgi_adapter
                .cast()
                .map_err(|e| WinError::new(e, "IDXGIAdapter::cast"))?
        };

        // Get the d3d device.
        let d3d_device = {
            let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
                .map_err(|e| WinError::new(e, "CreateDirect3D11DeviceFromDXGIDevice"))?;

            inspectable
                .cast()
                .map_err(|e| WinError::new(e, "IInspectable::cast"))?
        };

        Ok(Self {
            dxgi_adapter,
            d3d_device,
            d3d11_device,
            d3d11_context,
        })
    }
}

impl Drop for DirectX {
    fn drop(&mut self) {
        if let Err(e) = self.d3d_device.Close() {
            error!("Failed to close D3D device:\n{e}");
        }
    }
}

fn d3d11_device_with_type(
    driver_type: D3D_DRIVER_TYPE,
    flags: D3D11_CREATE_DEVICE_FLAG,
    device: *mut Option<ID3D11Device>,
) -> WindowsResult<()> {
    unsafe {
        D3D11CreateDevice(
            None,
            driver_type,
            HMODULE::default(),
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(device),
            None,
            None,
        )
    }
}
