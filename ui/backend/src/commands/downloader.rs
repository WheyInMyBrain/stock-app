// stock-app/ui/backend/src/commands/downloader.rs

use tauri::{AppHandle, command, Manager, Emitter};
use tauri_plugin_shell::ShellExt; 
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // 🎯 Added for lightning-fast socket binary framing reads
use crate::pipeline::WorkspaceDataContext;
use crate::commands::data_dir::get_active_data_directory;

// 🎯 GLOBAL PRIVATE HOLDER FOR THE PERSISTENT BACKGROUND GO SERVICE WRITER PIPE
static GO_PROCESS_WRITER: Mutex<Option<CommandChild>> = Mutex::new(None);

/// 🎯 APP INITIALIZATION GATEWAY: Boots the persistent Go binary and primes network session cookies
pub fn initialize_persistent_go_daemon(app_handle: AppHandle) {
    let current_app = app_handle.clone();
    
    tauri::async_runtime::spawn(async move {
        let active_dir = get_active_data_directory(current_app.clone());
        println!("🔥 [TAURI CORE DAEMON]: Spawning persistent Go Sidecar. Target dir: {}", active_dir);

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

                // 🚀 LIVE STDOUT STREAM MONITORING: Handles clean console logs exclusively!
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(bytes) => {
                            let stdout_chunk = String::from_utf8_lossy(&bytes).to_string();
                            
                            for line in stdout_chunk.lines() {
                                // Keep color markers and print terminal logs safely
                                println!("\x1b[34m{}\x1b[0m", line);

                                // Intercept the traditional completion signal safely outside standard data pipes
                                if line.starts_with("SIGNAL_COMPLETED:") {
                                    let parts: Vec<&str> = line.trim().split(':').collect();
                                    if parts.len() >= 3 {
                                        let ticker = parts[1];
                                        let api_name = parts[2];

                                        println!("\x1b[32m⚡ [DAEMON RECEPTION]: Go asset sync pipeline complete for [{}].\x1b[0m", ticker);

                                        WorkspaceDataContext::invalidate_ticker(ticker);

                                        if let Some(window) = current_app.get_webview_window("main") {
                                            #[derive(Clone, serde::Serialize)]
                                            struct GenericPayload { module_id: String, ticker: String }

                                            let target_module = if api_name == "symbol-core-data" { "stock_stats" } else { "company_profile" };

                                            let _ = window.emit(
                                                "pipeline-invalidated",
                                                GenericPayload { module_id: target_module.to_string(), ticker: ticker.to_string() }
                                            );
                                            let _ = window.emit(
                                                "pipeline-invalidated",
                                                GenericPayload { module_id: "company_profile".to_string(), ticker: ticker.to_string() }
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        CommandEvent::Stderr(bytes) => {
                            print!("\x1b[31m🚨 [GO DAEMON ERROR]: {}\x1b[0m", String::from_utf8_lossy(&bytes));
                        }
                        CommandEvent::Terminated(payload) => {
                            println!("\x1b[31m🏁 [GO DAEMON ALERT]: Background sidecar closed with status: {:?}\x1b[0m", payload.code);
                        }
                        _ => {}
                    }
                }
            }
            Err(err) => println!("❌ [CRITICAL DAEMON FAULT]: System failed to map persistent sidecar channels: {}", err),
        }
    });
}

/// 🎯 INTERCEPTOR INTERFACE LAYER
/// Transparently handles stream parameter parsing without breaking your standard ticker.rs pipeline mapping arrays.
#[command]
pub async fn run_sidecar_downloader(
    app_handle: AppHandle, 
    _data_dir_override: Option<String>,
    extra_args: Option<Vec<String>>, 
) -> Result<String, String> {
    
    let flags = extra_args.ok_or_else(|| "Missing execution flags matrix wrapper payload".to_string())?;
    if flags.is_empty() {
        return Err("Execution flags array size parsed empty".to_string());
    }

    let ticker = &flags[0];
    let mut mode = "both".to_string();
    let mut api = "".to_string();
    let mut is_stream_requested = false;

    // Sift dynamic parameters out smoothly
    for flag in &flags[1..] {
        if flag.starts_with("--mode=") {
            mode = flag.split('=').nth(1).unwrap_or("both").to_string();
        } else if flag.starts_with("--api=") {
            api = flag.split('=').nth(1).unwrap_or("").to_string();
        } else if flag == "--stream" {
            is_stream_requested = true;
        }
    }

    let active_dir = get_active_data_directory(app_handle.clone());

    // ============================================================================
    // 🎯 STEP 1: CACHE-FIRST APPROACH (IMMEDIATE PRE-RENDER SIGNAL DUMP)
    // ============================================================================
    if is_stream_requested && !api.is_empty() {
        let fallback_exchange = if mode == "both" { "nse" } else { &mode };
        let cached_file_path = std::path::Path::new(&active_dir)
            .join(ticker)
            .join(format!("{}_{}", fallback_exchange, api))
            .join("endpoint-metadata.json");

        if cached_file_path.exists() {
            if let Ok(cached_json_string) = std::fs::read_to_string(cached_file_path) {
                if let Some(window) = app_handle.get_webview_window("main") {
                    #[derive(Clone, serde::Serialize)]
                    struct DirectStreamPayload { module_id: String, ticker: String, raw_json: String }

                    let target_module = if api == "symbol-core-data" { "stock_stats" } else { "company_profile" };

                    println!("\x1b[35m🚀 [CACHE DISPATCH]: Found local snapshots for [{}]. Pre-rendering to UI instantly!\x1b[0m", ticker);

                    let _ = window.emit(
                        "live-memory-data", 
                        DirectStreamPayload {
                            module_id: target_module.to_string(),
                            ticker: ticker.to_string(),
                            raw_json: cached_json_string,
                        }
                    );
                }
            }
        }
    }

    // ============================================================================
    // 🎯 STEP 2: HIGH-PERFORMANCE INDUSTRIAL NETWORK SOCKET STREAM OVERLAP
    // ============================================================================
    if is_stream_requested {
        let socket_path = std::path::Path::new(&active_dir).join("downloader_engine.sock");
        let app_clone = app_handle.clone();
        let ticker_clone = ticker.to_string();
        let mode_clone = mode.clone();
        let api_clone = api.clone();

        // Spawn isolated network worker task thread context so UI thread execution is never blocked
        tauri::async_runtime::spawn(async move {
            println!("\x1b[36m📡 [IPC NETWORK DIAL]: Dialing local Unix Domain Socket channel file...\x1b[0m");
            
            match tokio::net::UnixStream::connect(&socket_path).await {
                Ok(mut socket) => {
                    let instruction_payload = format!("RUN {} {} {} --stream\n", ticker_clone, mode_clone, api_clone);
                    
                    // 1. Write request instruction directly over network pipeline link
                    if socket.write_all(instruction_payload.as_bytes()).await.is_ok() {
                        
                        // 2. Read Length-Prefixed Big-Endian 4-byte header descriptor integer
                        let mut header_buffer = [0u8; 4];
                        if socket.read_exact(&mut header_buffer).await.is_ok() {
                            let payload_size = u32::from_be_bytes(header_buffer) as usize;
                            
                            // 3. Allocate exact memory window array size instantly
                            let mut data_buffer = vec![0u8; payload_size];
                            if socket.read_exact(&mut data_buffer).await.is_ok() {
                                if let Ok(raw_json_string) = String::from_utf8(data_buffer) {
                                    println!("\x1b[32m⚡ [IPC STREAM SUCCESS]: Ingested {} bytes directly out of socket kernel frame memory!\x1b[0m", payload_size);
                                    
                                    // 4. Emit data structure to front-end window instantly
                                    if let Some(window) = app_clone.get_webview_window("main") {
                                        #[derive(Clone, serde::Serialize)]
                                        struct DirectStreamPayload { module_id: String, ticker: String, raw_json: String }

                                        let target_module = if api_clone == "symbol-core-data" { "stock_stats" } else { "company_profile" };

                                        let _ = window.emit(
                                            "live-memory-data", 
                                            DirectStreamPayload {
                                                module_id: target_module.to_string(),
                                                ticker: ticker_clone,
                                                raw_json: raw_json_string,
                                            }
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("\x1b[31m🚨 [IPC NETWORK CRITICAL ERR]: Failed connecting to hot Go socket server: {}\x1b[0m", e),
            }
        });

        return Ok("Signal dispatched via dedicated local network socket link successfully".to_string());
    }

    // ============================================================================
    // 🎯 STEP 3: STANDARD DISK FALLBACK ROUTING MATRIX (Heavy/OCR/Tasks)
    // ============================================================================
    let mut writer_guard = GO_PROCESS_WRITER.lock().unwrap();
    if let Some(ref mut child) = *writer_guard {
        let command_payload = format!("RUN {} {} {}\n", ticker, mode, api);
        
        println!("\x1b[33m📡 [INTERCEPTOR -> GO STDIN]: Routing fallback instruction payload down warm pipe: {}\x1b[0m", command_payload.trim());
        child.write(command_payload.as_bytes()).map_err(|e| e.to_string())?;
            
        Ok("Signal intercepted and dispatched successfully".to_string())
    } else {
        Err("Persistent background Go service layer is uninitialized or offline.".to_string())
    }
}