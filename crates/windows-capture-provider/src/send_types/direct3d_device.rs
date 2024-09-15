use std::ops::{Deref, DerefMut};

use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;

pub struct SendIDirect3DDevice(pub IDirect3DDevice);
unsafe impl Send for SendIDirect3DDevice {}
unsafe impl Sync for SendIDirect3DDevice {}

impl Deref for SendIDirect3DDevice {
    type Target = IDirect3DDevice;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SendIDirect3DDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
