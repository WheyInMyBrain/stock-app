// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command};
use tauri_plugin_shell::ShellExt; // 🎯 Tauri plugin tool to manage sidecar processes
use std::path::PathBuf;

#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle,
    ticker: String,
    mode: String,            // e.g., "profile", "chart", "all"
    data_dir_override: Option<String>,
) -> Result<String, String> {
    
    // 🎯 1. Resolve storage target path string safely
    let data_dir = data_dir_override
        .unwrap_or_else(|| {
            // Default fallback path mirroring your standard system repository anchor
            let base_dir = app_handle.path().app_local_data_dir().unwrap_or(PathBuf::from("../data"));
            base_dir.to_string_lossy().to_string()
        });

    println!("🔄 [TAURI SIDECAR BRIDGE]: Invoking Go Downloader for [{}] in mode [{}]. Target path: {}", ticker, mode, data_dir);

    // 🎯 2. Bind the arguments we built matching your Go --data-dir entry contract flags
    let args = vec![
        ticker.clone(),
        format!("--mode={}", mode),
        format!("--data-dir={}", data_dir),
    ];

    // 🎯 3. Resolve the sidecar binary dynamically from the backend compilation bundle folder context
    let sidecar = app_handle
        .shell()
        .sidecar("downloader") // Matches your tauri.conf.json identifier entry string perfectly
        .map_err(|e| format!("Failed to locate compiled sidecar resource track: {}", e))?
        .args(args);

    // 🎯 4. Execute the sidecar process and wait for its completion status
    let output = sidecar
        .output()
        .await
        .map_err(|e| format!("System crash sequence during sidecar execution: {}", e))?;

    // 🎯 5. Evaluate the outcome
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        println!("✅ [TAURI SIDECAR BRIDGE]: Go downloader completed data mining for [{}].", ticker);
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        println!("❌ [TAURI SIDECAR BRIDGE]: Go downloader errored: {}", stderr);
        Err(format!("Downloader execution failed: {}", stderr))
    }
}