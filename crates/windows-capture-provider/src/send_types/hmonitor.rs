use std::ops::{Deref, DerefMut};

use windows::Win32::Graphics::Gdi::HMONITOR;

/// Wrapper around an `HMONITOR` to make it `Send`.
#[derive(Debug, Clone, Copy)]
pub struct SendHMONITOR(pub HMONITOR);

unsafe impl Send for SendHMONITOR {}

impl Deref for SendHMONITOR {
    type Target = HMONITOR;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SendHMONITOR {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
