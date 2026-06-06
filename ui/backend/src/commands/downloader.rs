// stock-app/ui/backend/src/commands/downloader.rs
use std::process::{Stdio, Command};
use std::io::{BufRead, BufReader}; 
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; 
use crate::commands::data_loader::WorkspaceDataContext;
use crate::commands::data_dir::resolve_data_directory_headless;
use crate::commands::memory_pool::update_memory_cache;

pub fn initialize_persistent_go_daemon(_app_handle: tauri::AppHandle) {
    initialize_go_daemon();
}

/// 🚀 PURIFIED GO DAEMON SUMMONER
pub fn initialize_go_daemon() {
    std::thread::spawn(move || {
        let active_dir = resolve_data_directory_headless().to_string_lossy().to_string();
        let mut binaries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        binaries_dir.push("binaries");
        let mut sidecar_executable = None;

        if let Ok(entries) = std::fs::read_dir(binaries_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("downloader") {
                    sidecar_executable = Some(entry.path());
                    break;
                }
            }
        }

        let binary_path = match sidecar_executable {
            Some(p) => p,
            None => return,
        };

        let mut child = match Command::new(&binary_path)
            .args(&["--daemon", &format!("--data-dir={}", active_dir)])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() 
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let active_dir_clone = active_dir.clone();

        // Dedicated background thread listening to clear, light-blue Go logs
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout).lines();
            while let Some(Ok(line)) = reader.next() {
                let cleaned_line = strip_ansi_escape_codes(&line);
                println!("\x1b[96m[Go Output] {}\x1b[0m", cleaned_line);

                if line.starts_with("SIGNAL_COMPLETED:") {
                    let parts: Vec<&str> = line.trim().split(':').collect();
                    if parts.len() >= 3 {
                        let ticker = parts[1].to_uppercase();
                        let api = parts[2].to_string();

                        WorkspaceDataContext::invalidate_ticker(&ticker);

                        let file_path = Path::new(&active_dir_clone)
                            .join(&ticker)
                            .join(format!("nse_{}", api))
                            .join("endpoint-metadata.json");

                        if file_path.exists() {
                            if let Ok(raw_json) = std::fs::read_to_string(file_path) {
                                update_memory_cache(&ticker, &api, raw_json);
                            }
                        }
                    }
                }
            }
        });

        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr).lines();
            while let Some(Ok(line)) = reader.next() {
                println!("\x1b[31m🚨 [GO DAEMON ERROR]: {}\x1b[0m", line);
            }
        });

        let _ = child.wait();
    });
}

#[tauri::command]
pub async fn run_sidecar_downloader(
    _app_handle: tauri::AppHandle, 
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    let flags = extra_args.ok_or_else(|| "Missing execution flags".to_string())?;
    run_sidecar_downloader_native(flags).await
}

/// ⚡ UNIFIED IPC DISPATCHER
/// Channels 100% of transmissions across the Unix Domain Socket to trigger Go execution routines perfectly.
pub async fn run_sidecar_downloader_native(extra_args: Vec<String>) -> Result<String, String> {
    if extra_args.is_empty() { return Err("Flags empty".to_string()); }

    let active_dir = resolve_data_directory_headless().to_string_lossy().to_string();
    let socket_path = Path::new(&active_dir).join("downloader_engine.sock");

    match tokio::net::UnixStream::connect(&socket_path).await {
        Ok(mut socket) => {
            let mut payload_parts = vec!["RUN".to_string()];
            payload_parts.extend(extra_args.clone());
            
            let instruction_payload = payload_parts.join(" ") + "\n";
            
            if socket.write_all(instruction_payload.as_bytes()).await.is_ok() {
                // If a stream loop was explicitly requested anywhere in the argument matrix slice
                if extra_args.iter().any(|f| f == "--stream" || f == "-stream") {
                    let mut header_buffer = [0u8; 4];
                    if socket.read_exact(&mut header_buffer).await.is_ok() {
                        let payload_size = u32::from_be_bytes(header_buffer) as usize;
                        let mut data_buffer = vec![0u8; payload_size];
                        if socket.read_exact(&mut data_buffer).await.is_ok() {
                            if let Ok(raw_json_string) = String::from_utf8(data_buffer) {
                                // Dynamically detect ticker and api flags to update memory cache index slots seamlessly
                                let ticker = extra_args.iter().find(|f| !f.starts_with("-")).cloned().unwrap_or_default();
                                let api = extra_args.iter()
                                    .find(|f| f.starts_with("--api="))
                                    .map(|f| f.split('=').nth(1).unwrap_or(""))
                                    .unwrap_or("");
                                
                                if !ticker.is_empty() && !api.is_empty() {
                                    update_memory_cache(&ticker.to_uppercase(), api, raw_json_string.clone());
                                }
                                return Ok(raw_json_string);
                            }
                        }
                    }
                    return Err("Failed reading payload response from socket stream".to_string());
                } 
                
                // One-Shot Ingestion Pass
                return Ok("Signal accepted over socket layer successfully".to_string());
            }
            Err("Failed packing instructions down socket channel".to_string())
        }
        Err(e) => Err(format!("🚨 [IPC CONNECTION FAILED]: {}", e)),
    }
}

fn strip_ansi_escape_codes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while let Some(&next_c) = chars.peek() {
                chars.next();
                if next_c == 'm' { break; }
            }
        } else {
            result.push(c);
        }
    }
    result
}