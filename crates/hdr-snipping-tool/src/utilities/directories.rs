use std::fs::create_dir_all;

use super::failure::{Failure, Ignore};

/// Path to the screenshot directory.
pub fn screenshot_dir() -> std::path::PathBuf {
    let dir = dirs::picture_dir()
        .report_and_panic("The picture directory could not be retreived")
        .join("Screenshots");

    create_dir_all(&dir)
        .report("Could not create the screenshot directory")
        .ignore();

    dir
}

/// Path to the config directory.
pub fn config_dir() -> std::path::PathBuf {
    let dir = dirs::config_dir()
        .report_and_panic("The config directory could not be retreived")
        .join("HDR Snipping Tool");

    create_dir_all(&dir)
        .report("Could not create the config directory")
        .ignore();

    dir
}
