// stock-app/ui/backend/src/commands/ocr.rs
use std::process::{Stdio, Command};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use crate::commands::data_dir::resolve_data_directory_headless;

/// 🛠️ Bulletproof Binaries Directory Resolver: Checks both production paths 
/// and local development manifest directories to ensure paths never break.
fn resolve_binaries_directory() -> PathBuf {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("binaries");
        if exe_path.exists() {
            return exe_path;
        }
    }
    
    let mut manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_path.push("binaries");
    manifest_path
}

/// ⚡ STANDALONE FROZEN PYTHON OCR DISPATCHER
pub fn run_ocr_pipeline_command(
    symbol: &str,
    data_dir_override: Option<String>,
) -> Result<String, String> {
    let current_ticker = symbol.trim().to_uppercase();
    if current_ticker.is_empty() {
        return Err("❌ ERROR: Provided ticker symbol token cannot be blank.".to_string());
    }

    // A. Resolve base data repository coordinate path natively aligned with the workspace
    let final_dir = match data_dir_override {
        Some(dir) => dir,
        None => resolve_data_directory_headless().to_string_lossy().to_string(),
    };

    // B. Resolve binaries directory track and look up the target triple executable asset
    let binaries_dir = resolve_binaries_directory();
    let mut sidecar_executable = None;

    if let Ok(entries) = std::fs::read_dir(binaries_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("ocr-") {
                sidecar_executable = Some(entry.path());
                break;
            }
        }
    }

    let binary_path = match sidecar_executable {
        Some(p) => p,
        None => {
            return Err("🚨 [LAUNCH FAULT]: Standalone frozen Python OCR executable missing from binaries folder tracker.".to_string());
        }
    };

    println!("\x1b[35m[OCR] 🚀 [PROCESS ENGINE]: Triggering machine learning document extraction for ticker [{}]\x1b[0m", current_ticker);

    // C. Spawn the external frozen process with standard tracking pipes attached
    let mut child = Command::new(&binary_path)
        .arg(&current_ticker)
        .arg(format!("--data-dir={}", final_dir))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("🚨 Failed to initiate subprocess worker handle: {}", e))?;

    let stdout = child.stdout.take().ok_or_else(|| "Failed splitting process stdout channel pipe.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed splitting process stderr channel pipe.".to_string())?;

    // 🎯 FIXED OS PIPE DEADLOCK: Drain stdout and stderr concurrently in parallel threads
    let stdout_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout).lines();
        while let Some(Ok(line)) = reader.next() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                println!("\x1b[35m[OCR] {}\x1b[0m", trimmed);
            }
        }
    });

    let stderr_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr).lines();
        while let Some(Ok(err_line)) = reader.next() {
            let trimmed_err = err_line.trim();
            if !trimmed_err.is_empty() {
                println!("\x1b[35m[OCR] 🚨 [ENGINE SYSTEM ERROR]: {}\x1b[0m", trimmed_err);
            }
        }
    });

    // D. Wait for the drainage reader loops to conclude safely
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    // E. Wait for the processor routine thread contexts to conclude safely
    let status = child.wait().map_err(|e| format!("Failed awaiting sidecar child process loop closure: {}", e))?;

    if status.success() {
        Ok("Success".to_string())
    } else {
        Err("❌ [EXECUTION CRASH]: Python sidecar pipeline routine encountered an unmanaged termination state.".to_string())
    }
}