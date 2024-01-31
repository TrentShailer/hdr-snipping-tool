use windows::{
    core::{ComInterface, Result},
    Graphics::DirectX::Direct3D11::IDirect3DSurface,
    Win32::{
        Graphics::Direct3D11::ID3D11Texture2D,
        System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess,
    },
};

pub fn get_texture_from_surface(surface: &IDirect3DSurface) -> Result<ID3D11Texture2D> {
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let texture = unsafe { access.GetInterface::<ID3D11Texture2D>()? };
    Ok(texture)
}
