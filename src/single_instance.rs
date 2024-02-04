use windows::{
    core::{HRESULT, PCWSTR},
    Win32::System::Threading::{CreateMutexW, OpenMutexW, MUTEX_ALL_ACCESS},
};

const MUTEX_NAME: &str = "HDR-Snipping-Tool-Process-Mutex\0";

pub fn is_first_instance() -> anyhow::Result<bool> {
    unsafe {
        let result = OpenMutexW(
            MUTEX_ALL_ACCESS,
            true,
            PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr()),
        );

        // Some mutex with that name already exists, we aren't the first instance
        if result.is_ok() {
            return Ok(false);
        }

        let err = result.err().unwrap();
        // 0x80070002 is the error code for if no mutex with that names exists, any other error should be reported
        if err.code() != HRESULT(0x80070002u32 as i32) {
            return Err(err.into());
        }

        // Since no mutex exists, we should create it
        CreateMutexW(
            None,
            true,
            PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr()),
        )?;
    }

    Ok(true)
}
