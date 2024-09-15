use std::ops::{Deref, DerefMut};

use windows::Win32::Foundation::HANDLE;

#[derive(Debug, Clone, Copy)]
pub struct SendHANDLE(pub HANDLE);

unsafe impl Send for SendHANDLE {}

impl Deref for SendHANDLE {
    type Target = HANDLE;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SendHANDLE {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
