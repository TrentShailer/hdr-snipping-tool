use thiserror::Error;
use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
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
use windows_core::Interface;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create d3d device:\n{0}")]
    D3DDevice(#[source] windows_result::Error),

    #[error("Failed to create d3d11 device:\n{0}")]
    D3D11Device(#[source] windows_result::Error),

    #[error("Failed to create d3d11 context:\n{0}")]
    D3D11Context(#[source] windows_result::Error),

    #[error("Failed to create dxgi device:\n{0}")]
    DXGIDevice(#[source] windows_result::Error),

    #[error("Failed to create dxgi adapter:\n{0}")]
    DXGIAdapter(#[source] windows_result::Error),

    #[error("Failed to create DirectX device")]
    NoDevice,
}

/// Creates the dxgi device, d3d device, and d3d context used to process the capture
pub(crate) fn create_d3d_devices() -> Result<
    (
        IDXGIDevice,
        IDXGIAdapter1,
        IDirect3DDevice,
        ID3D11Device,
        ID3D11DeviceContext,
    ),
    Error,
> {
    // create d3d device for capture item
    let d3d11_device = match create_d3d11_device().map_err(Error::D3D11Device)? {
        Some(v) => v,
        None => return Err(Error::NoDevice),
    };

    let d3d11_context =
        unsafe { d3d11_device.GetImmediateContext() }.map_err(Error::D3D11Context)?;

    let dxgi_device: IDXGIDevice = d3d11_device.cast().map_err(Error::DXGIDevice)?;
    let dxgi_adapter: IDXGIAdapter1 = unsafe {
        dxgi_device
            .GetAdapter()
            .map_err(Error::DXGIAdapter)?
            .cast()
            .map_err(Error::DXGIAdapter)?
    };

    let d3d_device = create_d3d_device(&dxgi_device).map_err(Error::D3DDevice)?;

    Ok((
        dxgi_device,
        dxgi_adapter,
        d3d_device,
        d3d11_device,
        d3d11_context,
    ))
}

fn create_d3d_device(dxgi_device: &IDXGIDevice) -> windows_result::Result<IDirect3DDevice> {
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(dxgi_device)? };

    inspectable.cast()
}

fn create_d3d11_device() -> Result<Option<ID3D11Device>, windows_result::Error> {
    let mut device = None;
    let mut result = create_d3d11_device_with_type(
        D3D_DRIVER_TYPE_HARDWARE,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        &mut device,
    );

    if let Err(error) = &result {
        if error.code() == DXGI_ERROR_UNSUPPORTED {
            result = create_d3d11_device_with_type(
                D3D_DRIVER_TYPE_WARP,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &mut device,
            );
        }
    }
    result?;

    Ok(device)
}

fn create_d3d11_device_with_type(
    driver_type: D3D_DRIVER_TYPE,
    flags: D3D11_CREATE_DEVICE_FLAG,
    device: *mut Option<ID3D11Device>,
) -> windows::core::Result<()> {
    unsafe {
        D3D11CreateDevice(
            None,
            driver_type,
            None,
            flags,
            None,
            D3D11_SDK_VERSION,
            Some(device),
            None,
            None,
        )
    }
}
