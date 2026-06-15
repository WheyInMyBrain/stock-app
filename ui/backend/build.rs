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
    target_output_dir.pop(); 
    target_output_dir.pop(); 
    target_output_dir.pop(); 
    target_output_dir.push("binaries"); 

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
            #[cfg(target_os = "windows")]
            let mut py_cmd = Command::new("python");
            #[cfg(not(target_os = "windows"))]
            let mut py_cmd = Command::new("python3");

            py_cmd.current_dir(&ocr_source_dir).args(&["-m", "venv", ".venv"]).status().unwrap();
        }

        #[cfg(target_os = "windows")]
        let pip_exe = venv_dir.join("Scripts\\pip.exe");
        #[cfg(not(target_os = "windows"))]
        let pip_exe = venv_dir.join("bin/pip");

        #[cfg(target_os = "windows")]
        let pyinstaller_exe = venv_dir.join("Scripts\\pyinstaller.exe");
        #[cfg(not(target_os = "windows"))]
        let pyinstaller_exe = venv_dir.join("bin/pyinstaller");

        Command::new(&pip_exe).current_dir(&ocr_source_dir).args(&["install", "--upgrade", "pip"]).status().unwrap();
        Command::new(&pip_exe).current_dir(&ocr_source_dir).args(&["install", "-r", "requirements.txt"]).status().unwrap();
        Command::new(&pip_exe).current_dir(&ocr_source_dir).args(&["install", "pyinstaller"]).status().unwrap();

        Command::new(&pyinstaller_exe)
            .current_dir(&ocr_source_dir)
            .args(&[
                "--onefile", "--clean",
                "--recursive-copy-metadata", "docling",                   
                "--recursive-copy-metadata", "torch",                   
                "--recursive-copy-metadata", "huggingface_hub",                   
                "--distpath", target_output_dir.to_str().unwrap(),
                "--name", &format!("ocr-{}", target_triple), 
                "ocr_engine.py",
            ])
            .status().unwrap();
    }

    // =================================================================
    // 🤖 AUTOMATED C++ SIDECAR COMPILATION
    // =================================================================
    let ai_source_dir = manifest_dir.parent().unwrap().parent().unwrap().join("ai");
    let ai_src_folder = ai_source_dir.join("src");
    let cpp_build_dir = ai_source_dir.join("build");
    
    // Exact location of the compiled standalone executable target inside CMake's workspace
    let executable_internal_name = "ai_agent";
    let target_executable_internal_path = cpp_build_dir.join(executable_internal_name);

    // Dynamic filename for deployment context inside target/.../binaries/
    let cpp_sidecar_deploy_name = format!("ai_agent-{}{}", target_triple, ext);
    let final_cpp_sidecar_path = target_output_dir.join(&cpp_sidecar_deploy_name);

    // Watch your C++ directory for tracking code modifications
    watch_dir_recursively(&ai_src_folder, &[]);
    println!("cargo:rerun-if-changed={}", final_cpp_sidecar_path.display());

    let cpp_max_src_time = get_max_mtime(&ai_src_folder, &[]);
    let cpp_bin_time = final_cpp_sidecar_path.metadata().and_then(|m| m.modified()).ok();

    let must_rebuild_cpp = match (cpp_max_src_time, cpp_bin_time) {
        (Some(src_t), Some(bin_t)) => src_t > bin_t,
        _ => true, 
    };

    if must_rebuild_cpp {
        println!("cargo:warning= [AI BUILD]: Changes detected in C++ architecture. Compiling standalone sidecar agent...");
        
        if !cpp_build_dir.exists() {
            std::fs::create_dir_all(&cpp_build_dir).unwrap();
        }

        let cmake_status = Command::new("cmake")
            .current_dir(&cpp_build_dir)
            .arg("..")
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .status()
            .expect("Failed to execute CMake generation step.");

        if !cmake_status.success() {
            panic!("CMake metadata validation run failed.");
        }

        let make_status = Command::new("make")
            .current_dir(&cpp_build_dir)
            .arg("-j4")
            .status()
            .expect("Failed to execute Make build steps.");

        if !make_status.success() {
            panic!("C++ sidecar binary generation build run failed.");
        }

        // 🚀 THE CENTRALIZATION SHIPPER: Copies the freshly baked binary side-by-side into target/binaries/
        std::fs::copy(&target_executable_internal_path, &final_cpp_sidecar_path)
            .expect(" [AI BUILD ERROR]: Failed to route compiled sidecar executable to central binaries home directory.");
            
        println!("cargo:warning= [AI BUILD]: Standalone C++ sidecar deployed successfully -> {}", cpp_sidecar_deploy_name);
    } else {
        println!("cargo:warning= [AI BUILD]: Standalone C++ engine sidecar is fully up to date.");
    }
}