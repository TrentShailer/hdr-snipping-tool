use thiserror::Error;
use windows::{
    core::{HRESULT, HSTRING},
    Win32::System::Threading::{CreateMutexW, OpenMutexW, MUTEX_ALL_ACCESS},
};

const MUTEX_NAME: &str = "HDR-Snipping-Tool-Process-Mutex\0";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to open mutex:\n{0}")]
    Open(#[source] windows_result::Error),

    #[error("Failed to create mutex:\n{0}")]
    Create(#[source] windows_result::Error),
}

/// Checks if another instance of the app is running by using a Windows system mutex
pub fn ensure_only_instance() -> Result<bool, Error> {
    unsafe {
        let mutex_name = HSTRING::from(MUTEX_NAME);
        let open_result = OpenMutexW(MUTEX_ALL_ACCESS, true, &mutex_name);

        let open_error = match open_result {
            // Some mutex with that name already exists, we aren't the first instance
            Ok(_) => return Ok(false),
            Err(e) => e,
        };

        // 0x80070002 is the error code for if no mutex with that names exists, any other error should be reported
        if open_error.code() != HRESULT(0x80070002u32 as i32) {
            return Err(Error::Open(open_error));
        }

        // Since no mutex exists, we should create it
        CreateMutexW(None, true, &mutex_name).map_err(Error::Create)?;
    }

    Ok(true)
}
