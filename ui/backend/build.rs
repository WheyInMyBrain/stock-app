// stock-app/ui/backend/build.rs
use std::process::Command;
use std::path::PathBuf;

fn main() {
    // 1. 🎯 THE CACHE GUARDIAN: Tell Cargo to re-run this script ONLY if something 
    // inside the downloader directory changes. If no Go files change, this step is skipped!
    println!("cargo:rerun-if-changed=../../downloader");

    // 2. PROCESSOR SNIFFERS: Extract the target CPU architecture, target OS, 
    // and full Target Triple configuration variables from Cargo.
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(); // e.g., "aarch64", "x86_64"
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();     // e.g., "macos", "windows", "linux"
    let target_triple = std::env::var("TARGET").unwrap();       // e.g., "aarch64-apple-darwin"

    // 3. ARCHITECTURE MAPPER: Translate Rust target architectures to match Go environment values
    let goarch = match arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => &arch,
    };

    let goos = match os.as_str() {
        "macos" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        _ => &os,
    };

    // 4. PLATFORM RESOLUTION: Append file extensions safely if compiling for Windows targets
    let ext = if goos == "windows" { ".exe" } else { "" };
    let binary_name = format!("downloader-{}{}", target_triple, ext);

    // 5. PATH RECKONING: Resolve dynamic system folders relative to this crate's manifest space
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let go_source_dir = manifest_dir.join("../../downloader");
    let target_output_dir = manifest_dir.join("binaries");

    // Guarantee target binaries output directory layout structure is safely created on disk
    std::fs::create_dir_all(&target_output_dir).unwrap();
    let final_binary_path = target_output_dir.join(binary_name);

    println!("cargo:warning= [BUILD SCRIPT]: Auto-compiling Go sidecar for architecture: {} ({})", target_triple, goarch);

    // 6. ASYNC PROCESS DISPATCHER: Invoke 'go build' directly on host machine system tools
    let status = Command::new("go")
        .current_dir(&go_source_dir)
        .env("GOOS", goos)
        .env("GOARCH", goarch)
        .args(&[
            "build",
            "-ldflags=-s -w", // Shrinks Go binary size (strips debug symbols & symbol tables)
            "-o",
            final_binary_path.to_str().unwrap(),
            ".",
        ])
        .status()
        .expect("🚨 [BUILD ERROR]: Failed to launch 'go build' command process. Ensure Go is installed on your system PATH environment.");

    // Break compile chain right away if Go compiler reports broken code segments
    if !status.success() {
        panic!("🚨 [BUILD FAULT]: Go source code compilation phase failed! Inspect your downloader modules for syntax faults.");
    }
}