use tracing::instrument;
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
        UI::WindowsAndMessaging::WM_APP,
    },
};
use windows_core::{Interface, HRESULT};
use windows_result::{Error as WindowsError, Result as WindowsResult};

#[instrument(skip_all, err)]
pub fn dxgi_adapter(dxgi_device: &IDXGIDevice) -> WindowsResult<IDXGIAdapter1> {
    let dxgi_adapter = unsafe { dxgi_device.GetAdapter()? };
    let dxgi_adapter_1 = dxgi_adapter.cast()?;

    Ok(dxgi_adapter_1)
}

#[instrument(skip_all, err)]
pub fn d3d_device(dxgi_device: &IDXGIDevice) -> WindowsResult<IDirect3DDevice> {
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(dxgi_device)? };
    let d3d_device = inspectable.cast()?;
    Ok(d3d_device)
}

#[instrument(skip_all, err)]
pub fn d3d11_context(d3d11_device: &ID3D11Device) -> WindowsResult<ID3D11DeviceContext> {
    let context = unsafe { d3d11_device.GetImmediateContext()? };
    Ok(context)
}

#[instrument(skip_all, err)]
pub fn d3d11_device() -> WindowsResult<ID3D11Device> {
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
    result?;

    let device =
        device.ok_or_else(|| WindowsError::new(HRESULT::from_win32(WM_APP), "Device was none"))?;

    Ok(device)
}

#[instrument(skip_all, err)]
pub fn d3d11_device_with_type(
    driver_type: D3D_DRIVER_TYPE,
    flags: D3D11_CREATE_DEVICE_FLAG,
    device: *mut Option<ID3D11Device>,
) -> WindowsResult<()> {
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
