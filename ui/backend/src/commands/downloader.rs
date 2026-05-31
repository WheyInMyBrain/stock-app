// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command, Emitter};
use tauri_plugin_shell::ShellExt; 
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; 
use crate::commands::data_loader::WorkspaceDataContext;
use crate::commands::data_dir::get_active_data_directory;

static GO_PROCESS_WRITER: Mutex<Option<CommandChild>> = Mutex::new(None);

#[derive(Clone, serde::Serialize)]
struct GlobalStreamPayload {
    module_id: String,
    ticker: String,
    raw_json: String,
}

#[derive(Clone, serde::Serialize)]
struct GlobalInvalidationPayload {
    module_id: String,
    ticker: String,
}

pub fn initialize_persistent_go_daemon(app_handle: AppHandle) {
    let current_app = app_handle.clone();
    
    tauri::async_runtime::spawn(async move {
        let active_dir = get_active_data_directory(current_app.clone());
        let sidecar_setup = current_app
            .shell()
            .sidecar("downloader")
            .unwrap()
            .args(vec!["--daemon".to_string(), format!("--data-dir={}", active_dir)]);

        match sidecar_setup.spawn() {
            Ok((mut rx, child_control)) => {
                {
                    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
                    *writer_guard = Some(child_control);
                }

                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let stdout_chunk = String::from_utf8_lossy(&bytes).to_string();
                            for line in stdout_chunk.lines() {
                                println!("\x1b[34m{}\x1b[0m", line);

                                // Expected Go Output format: SIGNAL_COMPLETED:TICKER:MODULE_ID
                                if line.starts_with("SIGNAL_COMPLETED:") {
                                    let parts: Vec<&str> = line.trim().split(':').collect();
                                    if parts.len() >= 3 {
                                        let ticker = parts[1].to_string();
                                        let module_id = parts[2].to_string();

                                        WorkspaceDataContext::invalidate_ticker(&ticker);

                                        // 🚀 Global emit: Whoever is tracking this module_id captures the trigger instantly
                                        let _ = current_app.emit("pipeline-invalidated", GlobalInvalidationPayload {
                                            module_id,
                                            ticker,
                                        });
                                    }
                                }
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            print!("\x1b[31m🚨 [GO DAEMON ERROR]: {}\x1b[0m", String::from_utf8_lossy(&bytes));
                        }
                        _ => {}
                    }
                }
            }
            Err(err) => println!("❌ [CRITICAL DAEMON FAULT]: {}", err),
        }
    });
}

#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle, 
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    let flags = extra_args.ok_or_else(|| "Missing execution flags".to_string())?;
    if flags.is_empty() { return Err("Flags empty".to_string()); }

    let ticker = &flags[0];
    let mut mode = "both".to_string();
    let mut api = "".to_string();
    let mut is_stream_requested = false;
    let mut module_id = "unknown_module".to_string(); // 🚀 THE ABSTRACT TRACKING TOKEN

    for flag in &flags[1..] {
        if flag.starts_with("--mode=") { mode = flag.split('=').nth(1).unwrap_or("both").to_string(); }
        else if flag.starts_with("--api=") { api = flag.split('=').nth(1).unwrap_or("").to_string(); }
        else if flag.starts_with("--module_id=") { module_id = flag.split('=').nth(1).unwrap_or("unknown_module").to_string(); }
        else if flag == "--stream" { is_stream_requested = true; }
    }

    let active_dir = get_active_data_directory(app_handle.clone());

    // 🎯 STEP 1: COMPLETELY ABSTRACT CACHE FETCHING
    if is_stream_requested && !api.is_empty() {
        let fallback_exchange = if mode == "both" { "nse" } else { &mode };
        let cached_file_path = std::path::Path::new(&active_dir)
            .join(ticker).join(format!("{}_{}", fallback_exchange, api)).join("endpoint-metadata.json");

        if cached_file_path.exists() {
            if let Ok(cached_json_string) = std::fs::read_to_string(cached_file_path) {
                // We broadcast globally using the original module_id passed down from ticker.rs!
                let _ = app_handle.emit("live-memory-data", GlobalStreamPayload {
                    module_id: module_id.clone(),
                    ticker: ticker.to_string(),
                    raw_json: cached_json_string,
                });
            }
        }
    }

    // 🎯 STEP 2: COMPLETELY ABSTRACT UNIX SOCKET STREAMING
    if is_stream_requested {
        let socket_path = std::path::Path::new(&active_dir).join("downloader_engine.sock");
        let app_clone = app_handle.clone();
        let ticker_clone = ticker.to_string();
        let mode_clone = mode.clone();
        let api_clone = api.clone();
        let module_clone = module_id.clone();

        tauri::async_runtime::spawn(async move {
            match tokio::net::UnixStream::connect(&socket_path).await {
                Ok(mut socket) => {
                    // 🚀 PASS-THROUGH KEY INJECTION: We instruct Go to attach our module token parameter to the stream return line!
                    let instruction_payload = format!("RUN {} {} {} --stream --metadata_module={}\n", ticker_clone, mode_clone, api_clone, module_clone);
                    
                    if socket.write_all(instruction_payload.as_bytes()).await.is_ok() {
                        let mut header_buffer = [0u8; 4];
                        if socket.read_exact(&mut header_buffer).await.is_ok() {
                            let payload_size = u32::from_be_bytes(header_buffer) as usize;
                            let mut data_buffer = vec![0u8; payload_size];
                            if socket.read_exact(&mut data_buffer).await.is_ok() {
                                if let Ok(raw_json_string) = String::from_utf8(data_buffer) {
                                    
                                    // 🚀 GLOBAL BROADCAST: Blindly mirrors the data straight back to whoever asked for it
                                    let _ = app_clone.emit("live-memory-data", GlobalStreamPayload {
                                        module_id: module_clone,
                                        ticker: ticker_clone,
                                        raw_json: raw_json_string,
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("🚨 [SOCKET ERROR]: {}", e),
            }
        });

        return Ok("Signal dispatched via dedicated socket link successfully".to_string());
    }

    // 🎯 STEP 3: FALLBACK ENGINE PASSTHROUGH
    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
    if let Some(ref mut child) = *writer_guard {
        let command_payload = format!("RUN {} {} {} --metadata_module={}\n", ticker, mode, api, module_id);
        child.write(command_payload.as_bytes()).map_err(|e| e.to_string())?;
        Ok("Signal dispatched via fallback engine".to_string())
    } else {
        Err("Daemon offline".to_string())
    }
}