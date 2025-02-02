use std::fs::create_dir_all;

/// Path to the screenshot directory.
pub fn screenshot_dir() -> std::path::PathBuf {
    dirs::picture_dir()
        .expect("The dirs library does not support this platform's picture dir.")
        .join("Screenshots")
}

/// Path to the config directory.
pub fn config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("The dirs library does not support this patform's config dir.")
        .join("HDR Snipping Tool")
}

pub fn create_dirs() -> std::io::Result<()> {
    create_dir_all(config_dir())?;
    create_dir_all(screenshot_dir())?;

    Ok(())
}
