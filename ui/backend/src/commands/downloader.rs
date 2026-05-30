// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command, Manager};
use tauri_plugin_shell::ShellExt; 
use tauri_plugin_shell::process::CommandEvent; 
use std::path::PathBuf;

#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle,
    data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    
    let data_dir = data_dir_override
        .unwrap_or_else(|| {
            let base_dir = app_handle.path().app_local_data_dir().unwrap_or(PathBuf::from("../data"));
            base_dir.to_string_lossy().to_string()
        });

    println!("🔄 [TAURI SIDECAR BRIDGE]: Invoking passive Go Downloader. Target base directory: {}", data_dir);

    let mut args = Vec::new();

    args.push(format!("--data-dir={}", data_dir));

    if let Some(custom_flags) = extra_args {
        if !custom_flags.is_empty() {
            let ticker = &custom_flags[0];
            let configuration_flags = &custom_flags[1..];

            // 1. Attach the configuration arguments first (e.g., --mode=nse, --api=symbol-core-data)
            args.extend(configuration_flags.to_vec());

            // 2. Attach the ticker at the absolute end
            args.push(ticker.clone());
        }
    }

    println!("📡 [TAURI SIDECAR BRIDGE]: Final execution flags being sent to sidecar: {:?}", args);

    let sidecar = app_handle
        .shell()
        .sidecar("downloader") 
        .map_err(|e| format!("Failed to locate compiled sidecar resource track: {}", e))?
        .args(args);

    let (mut rx, _child) = sidecar
        .spawn()
        .map_err(|e| format!("System crash sequence during sidecar spawn: {}", e))?;

    let mut full_stdout = String::new();
    let mut full_stderr = String::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line_bytes) => {
                if let Ok(line_str) = String::from_utf8(line_bytes) {
                    print!("📟 [GO SIDECAR STDOUT]: {}", line_str);
                    full_stdout.push_str(&line_str);
                }
            }
            CommandEvent::Stderr(line_bytes) => {
                if let Ok(line_str) = String::from_utf8(line_bytes) {
                    print!("🚨 [GO SIDECAR STDERR]: {}", line_str);
                    full_stderr.push_str(&line_str);
                }
            }
            CommandEvent::Terminated(payload) => {
                println!("🏁 [GO SIDECAR]: Process terminated with exit code: {:?}", payload.code);
            }
            _ => {}
        }
    }

    println!("✅ [TAURI SIDECAR BRIDGE]: Go downloader execution stream completed successfully.");
    Ok(full_stdout)
}