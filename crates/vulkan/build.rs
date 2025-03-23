//! # Vulkan `build.rs`
//! * Copies the files in `./layers` to the target directory.
//!

use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    let layers_dir = crate_dir.join("layers");
    if !layers_dir.is_dir() {
        println!(
            "cargo::error=Directory '{}' does not exist",
            crate_dir.to_string_lossy()
        );
        return;
    }

    for entry in fs::read_dir(layers_dir).unwrap() {
        let entry = entry.unwrap();

        if entry.file_type().expect("File must have type").is_dir() {
            continue;
        }

        let out_file = target_dir.join(entry.file_name());

        fs::copy(entry.path(), out_file).unwrap();
    }
}
