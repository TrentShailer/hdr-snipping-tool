use tracing::{error, warn};
use windows::{
    Graphics::DirectX::Direct3D11::IDirect3DDevice,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE},
            Direct3D11::{
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
            },
            Dxgi::{
                CreateDXGIFactory1, DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED,
                DXGI_ERROR_DEVICE_RESET, DXGI_ERROR_DRIVER_INTERNAL_ERROR, DXGI_ERROR_INVALID_CALL,
                DXGI_ERROR_NOT_FOUND, DXGI_OUTPUT_DESC1, IDXGIAdapter1, IDXGIDevice1,
                IDXGIFactory1, IDXGIOutput, IDXGIOutput6,
            },
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
            unsafe {
                d3d11_device_with_type(
                    D3D_DRIVER_TYPE_HARDWARE,
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    &mut device,
                )
                .map_err(|e| WinError::new(e, "D3D11CreateDevice"))?
            };

            match device {
                Some(device) => device,
                None => unreachable!("D3D11 Device should be Some if no errors were returned"),
            }
        };

        // Get the d3d11 context.
        let d3d11_context = unsafe { d3d11_device.GetImmediateContext() }
            .map_err(|e| WinError::new(e, "ID3D11Device::GetImmediateContext"))?;

        // Cast to the dgxi device.
        let dxgi_device: IDXGIDevice1 = d3d11_device
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

    /// Returns if the DXGI Adapter is outdated (not current).
    pub fn dxgi_adapter_outdated(&self) -> Result<bool, windows_result::Error> {
        let dxgi_factory: IDXGIFactory1 = unsafe { self.dxgi_adapter.GetParent() }?;
        Ok(!unsafe { dxgi_factory.IsCurrent() }.as_bool())
    }

    /// Recreates the DXGI Adapter. If no DXGI Adapter with a matching LUID could be found, this
    /// will do nothing and return false.
    pub fn recreate_dxgi_adapter(&mut self) -> Result<bool, WinError> {
        let old_luid = unsafe { self.dxgi_adapter.GetDesc1() }
            .map_err(|e| WinError::new(e, "IDXGIAdapter1::GetDesc1"))?
            .AdapterLuid;

        let factory = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }
            .map_err(|e| WinError::new(e, "CreateDXGIFactory1"))?;

        let mut new_adapter = None;
        let mut index = 0;
        loop {
            let result = unsafe { factory.EnumAdapters1(index) };

            let adapter = match result {
                Ok(adapter) => adapter,
                Err(e) => {
                    if e.code() == DXGI_ERROR_NOT_FOUND {
                        break;
                    } else {
                        return Err(WinError::new(e, "IDXGIFactory1::EnumAdapters1"));
                    }
                }
            };

            let new_luid = unsafe { adapter.GetDesc1() }
                .map_err(|e| WinError::new(e, "IDXGIAdapter1::GetDesc1"))?
                .AdapterLuid;

            if new_luid == old_luid {
                new_adapter = Some(adapter);
                break;
            }

            index += 1;
        }

        match new_adapter {
            Some(adapter) => {
                self.dxgi_adapter = adapter;
                Ok(true)
            }
            None => {
                warn!(
                    "DXGI Adapter recreation could not find a DXGI adapter with an LUID matching the old DXGI Adapter."
                );
                Ok(false)
            }
        }
    }

    /// Enumerate the DXGI outputs using `IDXGIAdapter1::EnumOutputs` until `DXGI_ERROR_NOT_FOUND`.
    pub fn dxgi_outputs(&self) -> Result<Vec<IDXGIOutput>, windows_result::Error> {
        let mut outputs = Vec::new();

        let mut index = 0;
        loop {
            let result = unsafe { self.dxgi_adapter.EnumOutputs(index) };

            // break on `DXGI_ERROR_NOT_FOUND`, propegate other errors.
            let dxgi_output = match result {
                Ok(output) => output,
                Err(e) => {
                    if e.code() == DXGI_ERROR_NOT_FOUND {
                        break;
                    } else {
                        return Err(e);
                    }
                }
            };

            outputs.push(dxgi_output);

            index += 1;
        }

        Ok(outputs)
    }

    /// Returns the DXGI_OUTPUT_DESC1 for each IDXGIOutput.
    pub fn dxgi_output_descriptors(&self) -> Result<Vec<DXGI_OUTPUT_DESC1>, WinError> {
        self.dxgi_outputs()
            .map_err(|e| WinError::new(e, "IDXGIAdapter1::EnumOutputs"))?
            .into_iter()
            .map(|output| unsafe {
                output
                    .cast::<IDXGIOutput6>()
                    .map_err(|e| WinError::new(e, "IDXGIOutput::cast"))?
                    .GetDesc1()
                    .map_err(|e| WinError::new(e, "IDXGIOutput6::GetDesc1"))
            })
            .collect::<Result<_, _>>()
    }

    /// Returns if the d3d11 device is still valid.
    ///
    /// Logs an error with the reason it is invalid.
    pub fn devices_valid(&self) -> bool {
        if let Err(e) = unsafe { self.d3d11_device.GetDeviceRemovedReason() } {
            // Variants from https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11device-getdeviceremovedreason
            let reason = match e.code() {
                DXGI_ERROR_DEVICE_HUNG => "DXGI_ERROR_DEVICE_HUNG",
                DXGI_ERROR_DEVICE_REMOVED => "DXGI_ERROR_DEVICE_REMOVED",
                DXGI_ERROR_DEVICE_RESET => "DXGI_ERROR_DEVICE_RESET",
                DXGI_ERROR_DRIVER_INTERNAL_ERROR => "DXGI_ERROR_DRIVER_INTERNAL_ERROR",
                DXGI_ERROR_INVALID_CALL => "DXGI_ERROR_INVALID_CALL",

                code => unreachable!(
                    "ID3D11Device::GetDeviceRemovedReason should only return the values listed in https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11device-getdeviceremovedreason, but {code} was returned"
                ),
            };

            error!("D3D11 device was lost: {reason}");

            false
        } else {
            true
        }
    }
}

impl Drop for DirectX {
    fn drop(&mut self) {
        if let Err(e) = self.d3d_device.Close() {
            error!("Failed to close D3D device: {e}");
        }
    }
}

unsafe fn d3d11_device_with_type(
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
