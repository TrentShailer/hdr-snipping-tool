#[cfg(debug_assertions)]
pub const IS_DEV: bool = true;
#[cfg(not(debug_assertions))]
pub const IS_DEV: bool = false;

pub const DEBUG_ENV_VAR: &str = "hdr-snipping-tool-debug";

pub fn is_debug() -> bool {
    std::env::var(DEBUG_ENV_VAR).is_ok()
}

pub fn enable_debug() {
    std::env::set_var(DEBUG_ENV_VAR, "true");
}

pub fn is_verbose() -> bool {
    std::env::var(DEBUG_ENV_VAR).is_ok_and(|v| &v == "verbose")
}

pub fn enable_verbose() {
    std::env::set_var(DEBUG_ENV_VAR, "verbose");
}
