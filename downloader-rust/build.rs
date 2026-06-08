use std::process::Command;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let go_src_dir = manifest_dir.join("../downloader-go/scout");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let lib_path = out_dir.join("libgo_scout.a");

    println!("cargo:rerun-if-changed={}", go_src_dir.join("main.go").display());

    let status = Command::new("go")
        .current_dir(&go_src_dir)
        .args(&[
            "build",
            "-buildmode=c-archive",
            "-ldflags=-s -w",
            "-o",
            lib_path.to_str().unwrap(),
            ".",
        ])
        .status()
        .expect("Failed to launch Go compiler layer");

    if !status.success() {
        panic!("Go static library compilation failed");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=go_scout");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Security");
    }
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=pthread");
    }
}