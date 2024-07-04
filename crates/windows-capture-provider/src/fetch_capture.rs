use windows::{
    Graphics::Capture::Direct3D11CaptureFrame,
    Win32::{
        Graphics::Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D,
            D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC,
            D3D11_USAGE_STAGING,
        },
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};
use windows_core::Interface;
use winit::dpi::PhysicalSize;

pub(crate) fn fetch_capture(
    frame: Direct3D11CaptureFrame,
    d3d_device: &ID3D11Device,
    d3d_context: &ID3D11DeviceContext,
) -> windows_result::Result<(Vec<u8>, PhysicalSize<u32>)> {
    // Get the frame we captured
    let surface = frame.Surface()?;
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
        unsafe { d3d_device.CreateTexture2D(&staging_desc, None, Some(&mut texture))? };

        match texture {
            Some(v) => v,
            None => {
                return Err(windows_result::Error::new(
                    windows_core::HRESULT(-1),
                    "Failed to create the staging texture",
                ))
            }
        }
    };

    // Copy from to the staging texture
    unsafe {
        d3d_context.CopyResource(
            Some(&staging_texture.cast()?),
            Some(&source_texture.cast()?),
        )
    };

    // Map the staging texture to allow CPU read
    let staging_resource: ID3D11Resource = staging_texture.cast()?;
    let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe {
        d3d_context.Map(
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

    // TODO could add a check to skip this step
    // DirectX may add padding onto the width of the image for better alignment
    // To remove this, we copy the relevant data we want from each row to a new vec
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

    // let size = PhysicalSize::new(mapped_resource.RowPitch / 4 / 2, staging_desc.Height);
    let size = PhysicalSize::new(staging_desc.Width, staging_desc.Height);

    unsafe { d3d_context.Unmap(Some(&staging_resource), 0) };

    Ok((output_vec, size))
}
