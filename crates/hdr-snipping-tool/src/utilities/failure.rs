use tracing::error;
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND,
};

use crate::utilities::windows_helpers::display_message;

pub fn log_and_panic<Err: core::fmt::Display>(error: Err, message: &str) -> ! {
    error!("{message}: {error}");

    panic!("{message}: {error}");
}

pub fn report_and_panic<Err: core::fmt::Display>(error: Err, message: &str) -> ! {
    error!("{message}: {error}");

    let user_message =
        format!("ERROR:\n{message}.\nSee the logs for more details, the application will exit.");
    display_message(&user_message, MB_ICONERROR | MB_OK | MB_SETFOREGROUND);

    panic!("{message}: {error}");
}

pub fn report<Err: core::fmt::Display>(error: Err, message: &str) {
    error!("{message}: {error}");

    let user_message = format!("{message}.\nSee the logs for more details.");
    display_message(&user_message, MB_ICONWARNING | MB_OK | MB_SETFOREGROUND);
}

#[allow(unused)]
pub trait Failure<T> {
    fn report_and_panic(self, message: &str) -> T;
    fn report(self, message: &str) -> Option<T>;
    fn log_and_panic(self, message: &str) -> T;
}

impl<T, E: core::fmt::Display> Failure<T> for Result<T, E> {
    fn report_and_panic(self, message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => report_and_panic(error, message),
        }
    }

    fn report(self, message: &str) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(error) => {
                report(error, message);
                None
            }
        }
    }

    fn log_and_panic(self, message: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => log_and_panic(error, message),
        }
    }
}

impl<T> Failure<T> for Option<T> {
    fn report_and_panic(self, message: &str) -> T {
        match self {
            Some(value) => value,
            None => report_and_panic("Was None", message),
        }
    }

    fn report(self, message: &str) -> Self {
        match self {
            Some(value) => Some(value),
            None => {
                report("Was None", message);
                None
            }
        }
    }

    fn log_and_panic(self, message: &str) -> T {
        match self {
            Some(value) => value,
            None => log_and_panic("Was None", message),
        }
    }
}
