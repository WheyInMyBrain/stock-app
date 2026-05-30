// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command, Manager};
use tauri_plugin_shell::ShellExt; 
use std::path::PathBuf;

#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle,
    data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, // 🎯 PASS EVERYTHING ELSE HERE: ticker, --mode, --api, etc.
) -> Result<String, String> {
    
    // 1. Resolve storage target path string safely
    let data_dir = data_dir_override
        .unwrap_or_else(|| {
            let base_dir = app_handle.path().app_local_data_dir().unwrap_or(PathBuf::from("../data"));
            base_dir.to_string_lossy().to_string()
        });

    println!("🔄 [TAURI SIDECAR BRIDGE]: Invoking passive Go Downloader. Target base directory: {}", data_dir);

    // 2. Establish the primary required anchor argument
    let mut args = vec![
        format!("--data-dir={}", data_dir),
    ];

    // 3. 🎯 DYNAMIC EXTRA PASSTHROUGH: Append any extra parameters explicitly requested by the caller
    if let Some(custom_flags) = extra_args {
        args.extend(custom_flags);
    }

    println!("📡 [TAURI SIDECAR BRIDGE]: Final execution flags being sent to sidecar: {:?}", args);

    // 4. Resolve the sidecar binary dynamically from the backend bundle configuration
    let sidecar = app_handle
        .shell()
        .sidecar("downloader") 
        .map_err(|e| format!("Failed to locate compiled sidecar resource track: {}", e))?
        .args(args);

    // 5. Execute the sidecar process passively and await its absolute completion status
    let output = sidecar
        .output()
        .await
        .map_err(|e| format!("System crash sequence during sidecar execution: {}", e))?;

    // 6. Evaluate the outcome
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        println!("✅ [TAURI SIDECAR BRIDGE]: Go downloader execution completed successfully.");
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        println!("❌ [TAURI SIDECAR BRIDGE]: Go downloader execution failed: {}", stderr);
        Err(format!("Downloader execution failed: {}", stderr))
    }
}