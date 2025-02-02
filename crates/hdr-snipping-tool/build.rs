use std::env;

extern crate embed_resource;

fn main() {
    // Embed icon
    {
        println!("cargo:rerun-if-changed=icon.rc");
        println!("cargo:rerun-if-changed=../../media/icon.ico");

        embed_resource::compile("icon.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }

    // Link manifest
    {
        // From https://github.com/rust-lang/rust/blob/master/compiler/rustc/build.rs

        let target_os = env::var("CARGO_CFG_TARGET_OS");
        let target_env = env::var("CARGO_CFG_TARGET_ENV");

        if Ok("windows") == target_os.as_deref() && Ok("msvc") == target_env.as_deref() {
            static MANIFEST_FILE: &str = "manifest.xml";
            println!("cargo:rerun-if-changed={MANIFEST_FILE}");

            let mut manifest_path = env::current_dir().unwrap();
            manifest_path.push(MANIFEST_FILE);

            // Embed the Windows application manifest file.
            println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
            println!(
                "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
                manifest_path.to_str().unwrap()
            );
        }
    }
}
