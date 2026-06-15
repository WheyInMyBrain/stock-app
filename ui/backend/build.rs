use std::process::Command;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn watch_dir_recursively(dir: &Path, skip_dirs: &[&str]) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || skip_dirs.contains(&name) {
                        continue;
                    }
                }
                watch_dir_recursively(&path, skip_dirs);
            } else {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}

fn get_max_mtime(dir: &Path, skip_dirs: &[&str]) -> Option<SystemTime> {
    let mut max_time: Option<SystemTime> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || skip_dirs.contains(&name) {
                        continue;
                    }
                }
                if let Some(subdir_time) = get_max_mtime(&path, skip_dirs) {
                    max_time = Some(max_time.map_or(subdir_time, |t| t.max(subdir_time)));
                }
            } else {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(mtime) = metadata.modified() {
                        max_time = Some(max_time.map_or(mtime, |t| t.max(mtime)));
                    }
                }
            }
        }
    }
    max_time
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    
    let ocr_source_dir = manifest_dir.parent().unwrap().parent().unwrap().join("ocr");
    
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let mut target_output_dir = PathBuf::from(out_dir);
    target_output_dir.pop(); // pop out
    target_output_dir.pop(); // pop build identity hash
    target_output_dir.pop(); // pop "build" folder
    target_output_dir.push("binaries"); // target/[profile]/binaries

    std::fs::create_dir_all(&target_output_dir).unwrap();

    let ocr_skip = [".venv", "venv", "build", "dist", "__pycache__", "binaries"];

    watch_dir_recursively(&ocr_source_dir, &ocr_skip);

    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();     
    let target_triple = std::env::var("TARGET").unwrap();       

    let ext = if os == "windows" { ".exe" } else { "" };
    let ocr_binary_name = format!("ocr-{}{}", target_triple, ext);
    let final_ocr_path = target_output_dir.join(&ocr_binary_name);

    println!("cargo:rerun-if-changed={}", final_ocr_path.display());

    let ocr_max_src_time = get_max_mtime(&ocr_source_dir, &ocr_skip);
    let ocr_bin_time = final_ocr_path.metadata().and_then(|m| m.modified()).ok();

    let must_rebuild_ocr = match (ocr_max_src_time, ocr_bin_time) {
        (Some(src_t), Some(bin_t)) => src_t > bin_t,
        _ => true, 
    };

    if must_rebuild_ocr {
        println!("cargo:warning= [BUILD SCRIPT]: Source changes detected. Synchronizing environment and freezing OCR binary...");

        let venv_dir = ocr_source_dir.join(".venv");
        
        if !venv_dir.exists() {
            println!("cargo:warning= [BUILD SCRIPT]: .VENV missing. Spawning localized Python virtual environment...");
            #[cfg(target_os = "windows")]
            let mut py_cmd = Command::new("python");
            #[cfg(not(target_os = "windows"))]
            let mut py_cmd = Command::new("python3");

            let venv_status = py_cmd
                .current_dir(&ocr_source_dir)
                .args(&["-m", "venv", ".venv"])
                .status()
                .expect(" [BUILD ERROR]: Failed to invoke Python runtime to create virtual environment.");

            if !venv_status.success() {
                panic!(" [BUILD FAULT]: Failed creating local Python virtual environment sandbox.");
            }
        }

        #[cfg(target_os = "windows")]
        let pip_exe = venv_dir.join("Scripts\\pip.exe");
        #[cfg(not(target_os = "windows"))]
        let pip_exe = venv_dir.join("bin/pip");

        #[cfg(target_os = "windows")]
        let pyinstaller_exe = venv_dir.join("Scripts\\pyinstaller.exe");
        #[cfg(not(target_os = "windows"))]
        let pyinstaller_exe = venv_dir.join("bin/pyinstaller");

        println!("cargo:warning= [BUILD SCRIPT]: Checking and updating virtual environment library alignments...");
        let pip_status = Command::new(&pip_exe)
            .current_dir(&ocr_source_dir)
            .args(&["install", "--upgrade", "pip"])
            .status()
            .and_then(|_| {
                Command::new(&pip_exe)
                    .current_dir(&ocr_source_dir)
                    .args(&["install", "-r", "requirements.txt"])
                    .status()
            })
            .and_then(|_| {
                Command::new(&pip_exe)
                    .current_dir(&ocr_source_dir)
                    .args(&["install", "pyinstaller"])
                    .status()
            });

        if !pip_status.map_or(false, |s| s.success()) {
            panic!(" [BUILD FAULT]: Failed synchronizing required packages from requirements.txt down the venv track.");
        }

        let ocr_status = Command::new(&pyinstaller_exe)
            .current_dir(&ocr_source_dir)
            .args(&[
                "--onefile",
                "--clean",
                "--recursive-copy-metadata", "docling",                   
                "--recursive-copy-metadata", "torch",                   
                "--recursive-copy-metadata", "huggingface_hub",                   
                "--distpath", target_output_dir.to_str().unwrap(),
                "--name", &format!("ocr-{}", target_triple), 
                "ocr_engine.py",
            ])
            .status();

        match ocr_status {
            Ok(status) if status.success() => {
                println!("cargo:warning= [BUILD SCRIPT]: Python OCR engine successfully frozen -> {}", ocr_binary_name);
            }
            _ => {
                panic!(" [BUILD FAULT]: PyInstaller freezing pass failed! Inspect script errors.");
            }
        }
    } else {
        println!("cargo:warning= [BUILD SCRIPT]: Python OCR sidecar is completely up to date. Skipping freezing pass.");
    }
}