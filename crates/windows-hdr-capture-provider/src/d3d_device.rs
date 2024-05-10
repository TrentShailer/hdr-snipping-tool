use snafu::{ResultExt, Snafu};
use windows::{
    core::ComInterface,
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
            },
            Dxgi::{IDXGIDevice, DXGI_ERROR_UNSUPPORTED},
        },
        System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
    },
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Windows API call '{call}' returned an error."))]
    WindowsApi {
        source: windows::core::Error,
        call: &'static str,
    },
}

/// Creates the dxgi device, d3d device, and d3d context used to process the capture
pub fn create_d3d_devices() -> Result<(IDirect3DDevice, ID3D11Device, ID3D11DeviceContext), Error> {
    // create d3d device for capture item
    let d3d_device = create_d3d_device()?;
    let d3d_context = unsafe {
        d3d_device.GetImmediateContext().context(WindowsApiSnafu {
            call: "d3d_device
            .GetImmediateContext()",
        })?
    };

    let dxgi_device = create_dxgi_device(&d3d_device)?;

    Ok((dxgi_device, d3d_device, d3d_context))
}

fn create_dxgi_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice, Error> {
    let dxgi_device: IDXGIDevice = d3d_device.cast().context(WindowsApiSnafu {
        call: "d3d_deviec.cast()",
    })?;

    let inspectable = unsafe {
        CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device).context(WindowsApiSnafu {
            call: "CreateDirect3D11DeviceFromDXGIDevice",
        })?
    };

    inspectable.cast().context(WindowsApiSnafu {
        call: "inspectable.cast()",
    })
}

fn create_d3d_device() -> Result<ID3D11Device, Error> {
    let mut device = None;
    let mut result = create_d3d_device_with_type(
        D3D_DRIVER_TYPE_HARDWARE,
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        &mut device,
    );
    if let Err(error) = &result {
        if error.code() == DXGI_ERROR_UNSUPPORTED {
            result = create_d3d_device_with_type(
                D3D_DRIVER_TYPE_WARP,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &mut device,
            );
        }
    }
    result.context(WindowsApiSnafu {
        call: "D3D11CreateDevice",
    })?;

    Ok(device.unwrap())
}

fn create_d3d_device_with_type(
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
