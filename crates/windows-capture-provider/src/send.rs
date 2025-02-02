use windows::Win32::{
    Foundation::{HANDLE, HWND},
    Graphics::Gdi::HMONITOR,
};

/// A wrapper to make [HANDLE] [Send].
#[derive(Debug, Clone, Copy)]
pub struct SendHANDLE(pub HANDLE);
unsafe impl Send for SendHANDLE {}

/// A wrapper to make [HMONITOR] [Send].
#[derive(Debug, Clone, Copy)]
pub struct SendHMONITOR(pub HMONITOR);
unsafe impl Send for SendHMONITOR {}

/// A wrapper to make [HWND] [Send].
#[derive(Debug, Clone, Copy)]
pub struct SendHWND(pub HWND);
unsafe impl Send for SendHWND {}
