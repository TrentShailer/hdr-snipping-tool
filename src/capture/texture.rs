use anyhow::Context;
use windows::{
    core::ComInterface,
    Graphics::DirectX::Direct3D11::IDirect3DSurface,
    Win32::{
        Graphics::Direct3D11::ID3D11Texture2D,
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};

pub fn get_texture_from_surface(surface: &IDirect3DSurface) -> anyhow::Result<ID3D11Texture2D> {
    let access: IDirect3DDxgiInterfaceAccess = surface
        .cast()
        .context("Failed get cast surface to dgxi interface")?;
    let texture = unsafe {
        access
            .GetInterface::<ID3D11Texture2D>()
            .context("Failed to get texture from inferface")?
    };
    Ok(texture)
}
