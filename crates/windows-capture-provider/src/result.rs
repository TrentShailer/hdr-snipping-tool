use core::fmt::Display;

use thiserror::Error;
use windows::Win32::Foundation::WIN32_ERROR;
use windows_core::HRESULT;

/// A shortcut for `Result<T, WinError>`.
pub type LabelledWinResult<T> = Result<T, WinError>;

/// A Windows Result wrapped with some context for the call that triggered the error.
#[derive(Debug, Error)]
pub struct WinError {
    call: &'static str,
    #[source]
    source: WinErrorSource,
}

/// Possible sources for a WinError.
#[derive(Debug, Error)]
pub enum WinErrorSource {
    /// A [windows_result::Error].
    #[error(transparent)]
    WindowsError(#[from] windows_result::Error),

    /// An [HRESULT].
    #[error("HRESULT: {0}")]
    HResult(HRESULT),

    /// A [WIN32_ERROR].
    #[error("Win32: {0:?}")]
    Win32(WIN32_ERROR),

    /// A Windows NT Error.
    #[error("NT: {0}")]
    NT(i32),
}

impl WinError {
    /// Create a WinError from a `windows_result::Error` and a label.
    pub fn new(source: windows_result::Error, call: &'static str) -> Self {
        Self {
            call,
            source: source.into(),
        }
    }

    /// Create a new WinError from a `WIN32_ERROR` and a label.
    pub fn from_win32(source: WIN32_ERROR, call: &'static str) -> Self {
        Self {
            call,
            source: WinErrorSource::Win32(source),
        }
    }

    /// Create a new WinError from an NT error and a label.
    pub fn from_nt(source: i32, call: &'static str) -> Self {
        Self {
            call,
            source: WinErrorSource::NT(source),
        }
    }

    /// Create a new WinError from an `HRESULT` and a label.
    pub fn from_hresult(source: HRESULT, call: &'static str) -> Self {
        Self {
            call,
            source: WinErrorSource::HResult(source),
        }
    }
}

impl Display for WinError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Windows {} call failed:\n{}", self.call, self.source)
    }
}
