use tracing::{instrument, Level};
use windows::{
    Graphics::Capture::Direct3D11CaptureFrame,
    Win32::{
        Foundation::HANDLE,
        Graphics::{
            Direct3D11::{ID3D11Texture2D, D3D11_TEXTURE2D_DESC},
            Dxgi::{IDXGIResource1, DXGI_SHARED_RESOURCE_READ, DXGI_SHARED_RESOURCE_WRITE},
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};
use windows_core::Interface;
use windows_result::Error as WindowsError;

/// Retrieves the capture handle from the GPU.
#[instrument("retrieve_capture", level = Level::DEBUG, skip_all, err)]
pub fn retrieve_handle(d3d_capture: &Direct3D11CaptureFrame) -> Result<HANDLE, WindowsError> {
    // Get the surface of the capture
    let surface = d3d_capture.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let source_texture = unsafe { access.GetInterface::<ID3D11Texture2D>()? };

    let mut source_desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { source_texture.GetDesc(&mut source_desc) };

    // Create handle to shared texture
    let handle = unsafe {
        let shared_resource: IDXGIResource1 = source_texture.cast()?;
        shared_resource.CreateSharedHandle(
            None,
            (DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE).0,
            None,
        )?
    };

    Ok(handle)
}
