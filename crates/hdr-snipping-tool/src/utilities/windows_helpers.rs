use std::path::Path;

use windows::{
    Win32::{
        Foundation::{GetLastError, HWND, WIN32_ERROR},
        System::Threading::{CreateMutexW, MUTEX_ALL_ACCESS, OpenMutexW},
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                GetForegroundWindow, MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, MessageBoxW,
                SetForegroundWindow,
            },
        },
    },
    core::{HRESULT, HSTRING, h, w},
};
use windows_capture_provider::{LabelledWinResult, WinError};

/// Checks if another instance of the app is running by using a Windows system mutex.
pub fn is_first_instance() -> LabelledWinResult<bool> {
    let mutex_name = HSTRING::from("HDR-Snipping-Tool-Process-Mutex\0");

    // Check if the mutex was taken
    let mutex_taken = {
        const MUTEX_WASNT_TAKEN: i32 = 0x80070002u32 as i32;

        match unsafe { OpenMutexW(MUTEX_ALL_ACCESS, true, &mutex_name) } {
            Ok(_) => true,
            Err(error) => {
                // If the mutex wasn't taken return false
                if error.code() == HRESULT(MUTEX_WASNT_TAKEN) {
                    false
                } else {
                    return Err(WinError::new(error, "OpenMutexW"));
                }
            }
        }
    };

    if mutex_taken {
        return Ok(false);
    }

    // Since the mutex wasn't taken, this instance should take it
    unsafe { CreateMutexW(None, true, &mutex_name) }
        .map_err(|e| WinError::new(e, "CreateMutexW"))?;

    Ok(true)
}

/// Display a Windows message box.
pub fn display_message(message: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe {
        let message = HSTRING::from(message);

        MessageBoxW(None, &message, h!("HDR Snipping Tool"), style)
    }
}

/// Gets the handle to the current foreground window.
pub fn get_foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

/// Sets a window to be the foreground window.
pub fn set_foreground_window(handle: HWND) -> bool {
    if handle.is_invalid() {
        return false;
    }

    unsafe { SetForegroundWindow(handle).as_bool() }
}

/// Open a directory in the default file manager.
pub unsafe fn explore_directory(directory: &Path) -> Result<(), WIN32_ERROR> {
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("explore"),
            &HSTRING::from(directory),
            None,
            None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    // Success
    if result.0 as i64 > 32 {
        return Ok(());
    }

    Err(unsafe { GetLastError() })
}
