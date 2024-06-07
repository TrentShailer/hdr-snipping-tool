extern crate embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=icon.rc");
    println!("cargo:rerun-if-changed=../../media/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
        println!("cargo:rerun-if-changed=manifest.xml");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");

        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            std::path::Path::new("manifest.xml")
                .canonicalize()
                .unwrap()
                .display()
        );
    }

    embed_resource::compile("icon.rc", embed_resource::NONE);
}
