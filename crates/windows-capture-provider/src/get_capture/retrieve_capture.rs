use std::time::Instant;

use windows::{
    Graphics::Capture::Direct3D11CaptureFrame,
    Win32::{
        Graphics::Direct3D11::{
            ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE,
            D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
        UI::WindowsAndMessaging::WM_APP,
    },
};
use windows_core::{Interface, HRESULT};
use windows_result::Error as WindowsError;

use crate::DirectXDevices;

/// Retrieves the capture from the GPU.
pub fn retrieve_capture(
    devices: &DirectXDevices,
    d3d_capture: Direct3D11CaptureFrame,
) -> Result<Box<[u8]>, WindowsError> {
    let start = Instant::now();

    // Get the surface of the capture
    let surface = d3d_capture.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let source_texture = unsafe { access.GetInterface::<ID3D11Texture2D>()? };

    // Setup staging texture descriptor
    let mut source_desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { source_texture.GetDesc(&mut source_desc) };
    let staging_desc = D3D11_TEXTURE2D_DESC {
        BindFlags: 0,
        MiscFlags: 0,
        Usage: D3D11_USAGE_STAGING,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        ..source_desc
    };

    // Create staging texture
    let staging_texture = {
        let mut texture = None;
        unsafe {
            devices
                .d3d11_device
                .CreateTexture2D(&staging_desc, None, Some(&mut texture))?
        };

        texture.ok_or(WindowsError::new(
            HRESULT::from_win32(WM_APP),
            "Failed to create the staging texture",
        ))?
    };

    // Copy from to the staging texture
    unsafe {
        devices.d3d11_context.CopyResource(
            Some(&staging_texture.cast()?),
            Some(&source_texture.cast()?),
        )
    };

    // Map the staging texture to allow CPU read
    let staging_resource: ID3D11Resource = staging_texture.cast()?;
    let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe {
        devices.d3d11_context.Map(
            Some(&staging_resource),
            0,
            D3D11_MAP_READ,
            0,
            Some(&mut mapped_resource),
        )?;
    };

    // Copy data to CPU
    let raw_slice = unsafe {
        std::slice::from_raw_parts(
            mapped_resource.pData as *const _,
            (staging_desc.Height * mapped_resource.RowPitch) as usize,
        )
    };

    let capture_width = mapped_resource.RowPitch / 4 / 2;

    // DirectX may add padding onto the width of the image for better alignment
    // To remove this, we copy the relevant data we want from each row to a new vec
    let capture = if capture_width != staging_desc.Width {
        let bytes_per_pixel = 8; // RGBAF16 = 8 bpp
        let width = staging_desc.Width as usize;
        let height = staging_desc.Height as usize;
        let mut output_vec = vec![0u8; width * height * bytes_per_pixel];

        let output_row_length = width * bytes_per_pixel;
        for row in 0..height {
            let data_begin = row * output_row_length;
            let data_end = (row + 1) * output_row_length;

            let slice_begin = row * mapped_resource.RowPitch as usize;
            let slice_end = slice_begin + output_row_length;

            output_vec[data_begin..data_end].copy_from_slice(&raw_slice[slice_begin..slice_end]);
        }

        output_vec.into_boxed_slice()
    } else {
        raw_slice.to_vec().into_boxed_slice()
    };

    unsafe { devices.d3d11_context.Unmap(Some(&staging_resource), 0) };

    log::debug!(
        "[retrieve_capture]
  {} bytes
  [TIMING] {}ms",
        capture.len(),
        start.elapsed().as_millis()
    );

    Ok(capture)
}
